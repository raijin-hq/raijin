use std::collections::HashMap;

use inazuma::HighlightStyle;

/// Syntax highlighting theme mapping scope names to highlight styles.
///
/// Scope names follow TextMate/Tree-sitter conventions (e.g. "keyword", "string", "comment").
#[derive(Clone, Debug)]
pub struct SyntaxTheme {
    highlights: HashMap<String, HighlightStyle>,
}

impl SyntaxTheme {
    /// Creates a new syntax theme from a map of scope names to highlight styles.
    pub fn new(highlights: HashMap<String, HighlightStyle>) -> Self {
        Self { highlights }
    }

    /// Creates an empty syntax theme with no highlight rules.
    pub fn empty() -> Self {
        Self {
            highlights: HashMap::new(),
        }
    }

    /// Returns the highlight style for the given scope name, if any.
    pub fn get(&self, scope: &str) -> Option<&HighlightStyle> {
        self.highlights.get(scope)
    }

    /// Inserts a highlight style for the given scope name.
    pub fn insert(&mut self, scope: String, style: HighlightStyle) {
        self.highlights.insert(scope, style);
    }

    /// Returns an iterator over all scope name and highlight style pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &HighlightStyle)> {
        self.highlights.iter()
    }

    /// Returns the number of highlight rules.
    pub fn len(&self) -> usize {
        self.highlights.len()
    }

    /// Returns true if there are no highlight rules.
    pub fn is_empty(&self) -> bool {
        self.highlights.is_empty()
    }
}
