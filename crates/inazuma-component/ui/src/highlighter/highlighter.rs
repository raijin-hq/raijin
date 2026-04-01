use crate::highlighter::LanguageRegistry;

use anyhow::{Context, Result, anyhow};
use inazuma::SharedString;

use ropey::{ChunkCursor, Rope};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{
    collections::HashMap,
    ops::Range,
};
use tree_sitter::{
    InputEdit, ParseOptions, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

/// When a node spans more than this many bytes beyond the requested query
/// range, we recurse into its children instead of querying it directly.
pub(super) const LARGE_NODE_THRESHOLD: usize = 8 * 1024;

/// A syntax highlighter that supports incremental parsing, multiline text,
/// and caching of highlight results.
#[allow(unused)]
pub struct SyntaxHighlighter {
    language: SharedString,
    pub(super) query: Option<Query>,
    /// A separate query for injection patterns that have `#set! injection.combined`.
    pub(super) combined_injections_query: Option<Arc<Query>>,
    pub(super) injection_queries: HashMap<SharedString, Query>,

    locals_pattern_index: usize,
    highlights_pattern_index: usize,
    // highlight_indices: Vec<Option<Highlight>>,
    non_local_variable_patterns: Vec<bool>,
    injection_content_capture_index: Option<u32>,
    injection_language_capture_index: Option<u32>,
    combined_injection_content_capture_index: Option<u32>,
    local_scope_capture_index: Option<u32>,
    local_def_capture_index: Option<u32>,
    local_def_value_capture_index: Option<u32>,
    local_ref_capture_index: Option<u32>,

    /// The last parsed source text.
    pub(super) text: Rope,
    parser: Parser,
    /// The last parsed tree.
    pub(super) tree: Option<Tree>,

    /// Parsed injection trees (language → tree with ranges).
    /// These are built once in update() and queried multiple times in match_styles().
    pub(super) injection_layers: HashMap<SharedString, InjectionLayer>,
}

/// A parsed injection layer.
/// Stores the parsed tree and the ranges it covers.
pub(crate) struct InjectionLayer {
    pub(crate) tree: Tree,
}

/// Data needed to compute injection layers on a background thread.
pub(crate) struct InjectionParseData {
    pub(crate) query: Arc<Query>,
    pub(crate) content_capture_index: Option<u32>,
    /// Old injection trees for incremental re-parsing.
    pub(crate) old_layers: HashMap<SharedString, Tree>,
}

pub(super) struct TextProvider<'a>(pub(super) &'a Rope);
pub(super) struct ByteChunks<'a> {
    cursor: ChunkCursor<'a>,
    node_start: usize,
    node_end: usize,
    at_first: bool,
}
impl<'a> tree_sitter::TextProvider<&'a [u8]> for TextProvider<'a> {
    type I = ByteChunks<'a>;

    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        let range = node.byte_range();
        let cursor = self.0.chunk_cursor_at(range.start);

        ByteChunks {
            cursor,
            node_start: range.start,
            node_end: range.end,
            at_first: true,
        }
    }
}

impl<'a> Iterator for ByteChunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if !self.at_first {
            if !self.cursor.next() {
                return None;
            }
        }
        self.at_first = false;

        let chunk_byte_start = self.cursor.byte_offset();
        if chunk_byte_start >= self.node_end {
            return None;
        }

        let chunk = self.cursor.chunk().as_bytes();

        // Slice the chunk to only include bytes within the node's range.
        let start_in_chunk = self.node_start.saturating_sub(chunk_byte_start);
        let end_in_chunk = (self.node_end - chunk_byte_start).min(chunk.len());

        if start_in_chunk >= end_in_chunk {
            return None;
        }

        Some(&chunk[start_in_chunk..end_in_chunk])
    }
}

#[derive(Debug, Default, Clone)]
pub(super) struct HighlightSummary {
    count: usize,
    start: usize,
    end: usize,
    min_start: usize,
    max_end: usize,
}

/// The highlight item, the range is offset of the token in the tree.
#[derive(Debug, Default, Clone)]
pub(super) struct HighlightItem {
    /// The byte range of the highlight in the text.
    pub(super) range: Range<usize>,
    /// The highlight name, like `function`, `string`, `comment`, etc.
    pub(super) name: SharedString,
}

impl HighlightItem {
    pub(super) fn new(range: Range<usize>, name: impl Into<SharedString>) -> Self {
        Self {
            range,
            name: name.into(),
        }
    }
}

impl sum_tree::Item for HighlightItem {
    type Summary = HighlightSummary;
    fn summary(&self, _cx: &()) -> Self::Summary {
        HighlightSummary {
            count: 1,
            start: self.range.start,
            end: self.range.end,
            min_start: self.range.start,
            max_end: self.range.end,
        }
    }
}

impl sum_tree::Summary for HighlightSummary {
    type Context<'a> = &'a ();
    fn zero(_: Self::Context<'_>) -> Self {
        HighlightSummary {
            count: 0,
            start: usize::MIN,
            end: usize::MAX,
            min_start: usize::MAX,
            max_end: usize::MIN,
        }
    }

    fn add_summary(&mut self, other: &Self, _: Self::Context<'_>) {
        self.min_start = self.min_start.min(other.min_start);
        self.max_end = self.max_end.max(other.max_end);
        self.start = other.start;
        self.end = other.end;
        self.count += other.count;
    }
}

impl<'a> sum_tree::Dimension<'a, HighlightSummary> for usize {
    fn zero(_: &()) -> Self {
        0
    }

    fn add_summary(&mut self, _: &'a HighlightSummary, _: &()) {}
}

impl<'a> sum_tree::Dimension<'a, HighlightSummary> for Range<usize> {
    fn zero(_: &()) -> Self {
        Default::default()
    }

    fn add_summary(&mut self, summary: &'a HighlightSummary, _: &()) {
        self.start = summary.start;
        self.end = summary.end;
    }
}

impl SyntaxHighlighter {
    /// Create a new SyntaxHighlighter for HTML.
    pub fn new(lang: &str) -> Self {
        match Self::build_combined_injections_query(&lang) {
            Ok(result) => result,
            Err(err) => {
                tracing::warn!(
                    "SyntaxHighlighter init failed, fallback to use `text`, {}",
                    err
                );
                Self::build_combined_injections_query("text").unwrap()
            }
        }
    }

    /// Build the combined injections query for the given language.
    ///
    /// https://github.com/tree-sitter/tree-sitter/blob/v0.25.5/highlight/src/lib.rs#L336
    fn build_combined_injections_query(lang: &str) -> Result<Self> {
        let Some(config) = LanguageRegistry::singleton().language(&lang) else {
            return Err(anyhow!(
                "language {:?} is not registered in `LanguageRegistry`",
                lang
            ));
        };

        let mut parser = Parser::new();
        parser
            .set_language(&config.language)
            .context("parse set_language")?;

        // Concatenate the query strings, keeping track of the start offset of each section.
        let mut query_source = String::new();
        query_source.push_str(&config.injections);
        let locals_query_offset = query_source.len();
        query_source.push_str(&config.locals);
        let highlights_query_offset = query_source.len();
        query_source.push_str(&config.highlights);

        // Construct a single query by concatenating the three query strings, but record the
        // range of pattern indices that belong to each individual string.
        let mut query = Query::new(&config.language, &query_source).context("new query")?;

        let mut locals_pattern_index = 0;
        let mut highlights_pattern_index = 0;
        for i in 0..(query.pattern_count()) {
            let pattern_offset = query.start_byte_for_pattern(i);
            if pattern_offset < highlights_query_offset {
                if pattern_offset < highlights_query_offset {
                    highlights_pattern_index += 1;
                }
                if pattern_offset < locals_query_offset {
                    locals_pattern_index += 1;
                }
            }
        }

        // Separate combined injection patterns into their own query.
        // Combined injections (e.g., PHP's HTML text nodes) collect all matching
        // ranges and parse them as a single document, so that opening/closing
        // tags across injection boundaries are correctly matched.
        let combined_injections_query = if !config.injections.is_empty() {
            if let Ok(mut ciq) = Query::new(&config.language, &config.injections) {
                let mut has_combined_query = false;
                for pattern_index in 0..locals_pattern_index {
                    let settings = query.property_settings(pattern_index);
                    if settings.iter().any(|s| &*s.key == "injection.combined") {
                        has_combined_query = true;
                        query.disable_pattern(pattern_index);
                    } else {
                        ciq.disable_pattern(pattern_index);
                    }
                }
                if has_combined_query { Some(Arc::new(ciq)) } else { None }
            } else {
                None
            }
        } else {
            None
        };

        let combined_injection_content_capture_index =
            combined_injections_query.as_ref().and_then(|q| {
                q.capture_names()
                    .iter()
                    .position(|name| *name == "injection.content")
                    .map(|i| i as u32)
            });

        // Find all of the highlighting patterns that are disabled for nodes that
        // have been identified as local variables.
        let non_local_variable_patterns = (0..query.pattern_count())
            .map(|i| {
                query
                    .property_predicates(i)
                    .iter()
                    .any(|(prop, positive)| !*positive && prop.key.as_ref() == "local")
            })
            .collect();

        // Store the numeric ids for all of the special captures.
        let mut injection_content_capture_index = None;
        let mut injection_language_capture_index = None;
        let mut local_def_capture_index = None;
        let mut local_def_value_capture_index = None;
        let mut local_ref_capture_index = None;
        let mut local_scope_capture_index = None;
        for (i, name) in query.capture_names().iter().enumerate() {
            let i = Some(i as u32);
            match *name {
                "injection.content" => injection_content_capture_index = i,
                "injection.language" => injection_language_capture_index = i,
                "local.definition" => local_def_capture_index = i,
                "local.definition-value" => local_def_value_capture_index = i,
                "local.reference" => local_ref_capture_index = i,
                "local.scope" => local_scope_capture_index = i,
                _ => {}
            }
        }

        let mut injection_queries = HashMap::new();
        for inj_language in config.injection_languages.iter() {
            if let Some(inj_config) = LanguageRegistry::singleton().language(&inj_language) {
                match Query::new(&inj_config.language, &inj_config.highlights) {
                    Ok(q) => {
                        injection_queries.insert(inj_config.name.clone(), q);
                    }
                    Err(e) => {
                        tracing::error!(
                            "failed to build injection query for {:?}: {:?}",
                            inj_config.name,
                            e
                        );
                    }
                }
            }
        }

        // let highlight_indices = vec![None; query.capture_names().len()];

        Ok(Self {
            language: config.name.clone(),
            query: Some(query),
            combined_injections_query,
            injection_queries,

            locals_pattern_index,
            highlights_pattern_index,
            non_local_variable_patterns,
            injection_content_capture_index,
            injection_language_capture_index,
            combined_injection_content_capture_index,
            local_scope_capture_index,
            local_def_capture_index,
            local_def_value_capture_index,
            local_ref_capture_index,
            text: Rope::new(),
            parser,
            tree: None,
            injection_layers: HashMap::new(),
        })
    }

    pub fn is_empty(&self) -> bool {
        self.text.len() == 0
    }

    /// Get the parsed tree (if available)
    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }

    /// Returns the language name for this highlighter.
    pub fn language(&self) -> &SharedString {
        &self.language
    }

    /// Returns a reference to the current text.
    pub fn text(&self) -> &Rope {
        &self.text
    }

    /// Highlight the given text, returning a map from byte ranges to highlight captures.
    ///
    /// Uses incremental parsing by `edit` to efficiently update the highlighter's state.
    /// When `timeout` is `Some`, aborts if parsing exceeds the given duration
    /// and returns `false`. On timeout the old tree is preserved so highlighting
    /// still works with stale data, but `self.text` is updated so that the
    /// caller can send the current text to a background parse.
    /// When `timeout` is `None`, parsing runs to completion and always returns `true`.
    pub fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        timeout: Option<Duration>,
    ) -> bool {
        if self.text.eq(text) {
            return true;
        }

        let edit = edit.unwrap_or(InputEdit {
            start_byte: 0,
            old_end_byte: 0,
            new_end_byte: text.len(),
            start_position: Point::new(0, 0),
            old_end_position: Point::new(0, 0),
            new_end_position: Point::new(0, 0),
        });

        let mut old_tree = self
            .tree
            .take()
            .unwrap_or(self.parser.parse("", None).unwrap());
        old_tree.edit(&edit);

        let mut timed_out = false;
        let start = Instant::now();
        let mut progress = |_: &tree_sitter::ParseState| -> bool {
            let Some(budget) = timeout else {
                return false;
            };

            if start.elapsed() > budget {
                timed_out = true;
                return true; // Cancel execution
            }

            false
        };

        let options = ParseOptions::new().progress_callback(&mut progress);
        let new_tree = self.parser.parse_with_options(
            &mut move |offset, _| {
                if offset >= text.len() {
                    ""
                } else {
                    let (chunk, chunk_byte_ix) = text.chunk(offset);
                    &chunk[offset - chunk_byte_ix..]
                }
            },
            Some(&old_tree),
            Some(options),
        );

        if timed_out || new_tree.is_none() {
            // Restore the old tree so highlighting continues with stale data.
            self.tree = Some(old_tree);
            self.text = text.clone();
            return false;
        }

        let new_tree = new_tree.unwrap();
        self.tree = Some(new_tree.clone());
        self.text = text.clone();
        self.parse_combined_injections(&new_tree);
        true
    }

    /// Returns the data needed to compute injection layers on a background thread.
    /// Returns `None` if this language has no combined injections.
    pub(crate) fn injection_parse_data(&self) -> Option<InjectionParseData> {
        let query = self.combined_injections_query.clone()?;
        Some(InjectionParseData {
            query,
            content_capture_index: self.combined_injection_content_capture_index,
            old_layers: self
                .injection_layers
                .iter()
                .map(|(k, v)| (k.clone(), v.tree.clone()))
                .collect(),
        })
    }

    /// Compute injection layers from a freshly-parsed main tree.
    /// This is pure computation with no side effects and is safe to run on a
    /// background thread.
    pub(crate) fn compute_injection_layers(
        data: InjectionParseData,
        tree: &Tree,
        text: &Rope,
    ) -> HashMap<SharedString, InjectionLayer> {
        let root_node = tree.root_node();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&data.query, root_node, TextProvider(text));

        let mut combined_ranges: HashMap<SharedString, Vec<tree_sitter::Range>> = HashMap::new();
        while let Some(query_match) = matches.next() {
            let mut language_name: Option<SharedString> = None;
            if let Some(prop) = data
                .query
                .property_settings(query_match.pattern_index)
                .iter()
                .find(|prop| prop.key.as_ref() == "injection.language")
            {
                language_name = prop
                    .value
                    .as_ref()
                    .map(|v| SharedString::from(v.to_string()));
            }
            let Some(language_name) = language_name else {
                continue;
            };
            for capture in query_match
                .captures
                .iter()
                .filter(|cap| Some(cap.index) == data.content_capture_index)
            {
                combined_ranges
                    .entry(language_name.clone())
                    .or_default()
                    .push(capture.node.range());
            }
        }

        let mut new_layers = HashMap::new();
        for (language_name, ranges) in combined_ranges {
            if ranges.is_empty() {
                continue;
            }
            let Some(config) = LanguageRegistry::singleton().language(&language_name) else {
                continue;
            };
            let mut parser = Parser::new();
            if parser.set_language(&config.language).is_err() {
                continue;
            }
            if parser.set_included_ranges(&ranges).is_err() {
                continue;
            }
            let old_tree = data.old_layers.get(&language_name);
            let Some(new_tree) = parser.parse_with_options(
                &mut |offset, _| {
                    if offset >= text.len() {
                        ""
                    } else {
                        let (chunk, chunk_byte_ix) = text.chunk(offset);
                        &chunk[offset - chunk_byte_ix..]
                    }
                },
                old_tree,
                None,
            ) else {
                continue;
            };
            new_layers.insert(language_name, InjectionLayer { tree: new_tree });
        }
        new_layers
    }

    /// Apply a tree that was parsed on a background thread.
    ///
    /// `injection_layers` must also be pre-computed in the background via
    /// [`compute_injection_layers`] to avoid blocking the main thread.
    pub(crate) fn apply_background_tree(
        &mut self,
        tree: Tree,
        text: &Rope,
        injection_layers: HashMap<SharedString, InjectionLayer>,
    ) {
        // Only apply if the text still matches what was parsed.
        if !self.text.eq(text) {
            return;
        }

        self.tree = Some(tree);
        self.injection_layers = injection_layers;
    }

    /// Parse all combined injections after main tree is updated.
    /// pattern: parse once in update, query many times in render.
    /// Parse all combined injections after main tree is updated.
    /// pattern: parse once in update, query many times in render.
    fn parse_combined_injections(&mut self, tree: &Tree) {
        let Some(data) = self.injection_parse_data() else {
            return;
        };
        self.injection_layers = Self::compute_injection_layers(data, tree, &self.text.clone());
    }
}
