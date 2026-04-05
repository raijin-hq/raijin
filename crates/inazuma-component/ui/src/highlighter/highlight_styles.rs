use crate::highlighter::HighlightTheme;

use inazuma::{HighlightStyle, SharedString};

use std::{collections::BTreeSet, ops::Range};
use tree_sitter::{QueryCursor, StreamingIterator};

use super::highlighter::{
    HighlightItem, SyntaxHighlighter, TextProvider, LARGE_NODE_THRESHOLD,
};

/// To merge intersection ranges, let the subsequent range cover
/// the previous overlapping range and split the previous range.
///
/// From:
///
/// AA
///   BBB
///    CCCCC
///      DD
///         EEEE
///
/// To:
///
/// AABCCDDCEEEE
pub(crate) fn unique_styles(
    total_range: &Range<usize>,
    styles: Vec<(Range<usize>, HighlightStyle)>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    if styles.is_empty() {
        return styles;
    }

    // Create intervals: (position, is_start, style_index)
    let mut intervals: Vec<(usize, bool, usize)> = Vec::with_capacity(styles.len() * 2 + 2);
    for (i, (range, _)) in styles.iter().enumerate() {
        intervals.push((range.start, true, i));
        intervals.push((range.end, false, i));
    }

    intervals.push((total_range.start, true, usize::MAX));
    intervals.push((total_range.end, false, usize::MAX));

    // Sort by position, with ends before starts at same position
    // This ensures we close ranges before opening new ones at the same position
    intervals.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    // Track significant intervals (where style ranges end) for merging decisions
    let mut significant_intervals: BTreeSet<usize> = BTreeSet::new();
    for (range, _) in &styles {
        significant_intervals.insert(range.end);
    }

    let mut result: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    let mut active_styles: Vec<usize> = Vec::new();
    let mut last_pos = total_range.start;

    for (pos, is_start, style_idx) in intervals {
        // Skip total_range boundaries in active set management
        let is_boundary = style_idx == usize::MAX;

        if pos > last_pos {
            let interval = last_pos..pos;
            let combined_style = if active_styles.is_empty() {
                HighlightStyle::default()
            } else {
                let mut combined = HighlightStyle::default();
                for &idx in &active_styles {
                    merge_highlight_style(&mut combined, &styles[idx].1);
                }
                combined
            };

            result.push((interval, combined_style));
        }

        if !is_boundary {
            if is_start {
                active_styles.push(style_idx);
            } else {
                active_styles.retain(|&i| i != style_idx);
            }
        }

        last_pos = pos;
    }

    // Merge adjacent ranges with the same style, but not across significant boundaries
    let mut merged: Vec<(Range<usize>, HighlightStyle)> = Vec::with_capacity(result.len());
    for (range, style) in result {
        if let Some((last_range, last_style)) = merged.last_mut() {
            if last_range.end == range.start
                && *last_style == style
                && !significant_intervals.contains(&range.start)
            {
                // Merge adjacent ranges with same style, but not across significant boundaries
                last_range.end = range.end;
                continue;
            }
        }
        merged.push((range, style));
    }

    merged
}

/// Walk the tree and collect nodes suitable for querying, skipping subtrees
/// that fall entirely outside the byte range. Nodes much larger than the
/// query range are recursed into so that `QueryCursor` only visits the
/// relevant portion of the tree.
pub(super) fn collect_query_nodes<'a>(
    root: tree_sitter::Node<'a>,
    range: &Range<usize>,
) -> Vec<tree_sitter::Node<'a>> {
    let mut nodes = Vec::new();
    collect_query_nodes_inner(root, range, &mut nodes);
    if nodes.is_empty() {
        nodes.push(root);
    }
    nodes
}

fn collect_query_nodes_inner<'a>(
    node: tree_sitter::Node<'a>,
    range: &Range<usize>,
    out: &mut Vec<tree_sitter::Node<'a>>,
) {
    // Skip nodes entirely outside the range.
    if node.end_byte() <= range.start || node.start_byte() >= range.end {
        return;
    }

    let node_span = node.end_byte() - node.start_byte();
    let range_span = range.end - range.start;

    // Use `goto_first_child_for_byte` to seek directly to the first
    // overlapping child instead of iterating all children from the start.
    if node_span > range_span + LARGE_NODE_THRESHOLD && node.child_count() > 0 {
        let mut cursor = node.walk();
        if cursor.goto_first_child_for_byte(range.start).is_some() {
            loop {
                let child = cursor.node();
                if child.start_byte() >= range.end {
                    break;
                }
                collect_query_nodes_inner(child, range, out);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        return;
    }

    out.push(node);
}

/// Merge other style (Other on top)
pub(super) fn merge_highlight_style(style: &mut HighlightStyle, other: &HighlightStyle) {
    if let Some(color) = other.color {
        style.color = Some(color);
    }
    if let Some(font_weight) = other.font_weight {
        style.font_weight = Some(font_weight);
    }
    if let Some(font_style) = other.font_style {
        style.font_style = Some(font_style);
    }
    if let Some(background_color) = other.background_color {
        style.background_color = Some(background_color);
    }
    if let Some(underline) = other.underline {
        style.underline = Some(underline);
    }
    if let Some(strikethrough) = other.strikethrough {
        style.strikethrough = Some(strikethrough);
    }
    if let Some(fade_out) = other.fade_out {
        style.fade_out = Some(fade_out);
    }
}

impl SyntaxHighlighter {
    /// Match the visible ranges of nodes in the Tree for highlighting.
    pub(super) fn match_styles(&self, range: Range<usize>) -> Vec<HighlightItem> {
        let mut highlights = vec![];
        let Some(tree) = &self.tree else {
            return highlights;
        };

        let Some(query) = &self.query else {
            return highlights;
        };

        let root_node = tree.root_node();
        let source = &self.text;

        // Query pre-parsed injection layers.
        for (language_name, layer) in &self.injection_layers {
            let Some(query) = self.injection_queries.get(language_name) else {
                continue;
            };

            let mut query_cursor = QueryCursor::new();
            query_cursor.set_byte_range(range.clone());

            let mut matches =
                query_cursor.matches(query, layer.tree.root_node(), TextProvider(&self.text));

            let mut last_end = 0usize;
            while let Some(m) = matches.next() {
                for cap in m.captures {
                    let node_range = cap.node.start_byte()..cap.node.end_byte();

                    if node_range.start < last_end {
                        continue;
                    }

                    if let Some(highlight_name) = query.capture_names().get(cap.index as usize) {
                        last_end = node_range.end;
                        highlights.push(HighlightItem::new(
                            node_range,
                            SharedString::from(highlight_name.to_string()),
                        ));
                    }
                }
            }
        }

        let query_nodes = collect_query_nodes(root_node, &range);

        for query_node in &query_nodes {
            let mut query_cursor = QueryCursor::new();
            query_cursor.set_byte_range(range.clone());

            let mut matches = query_cursor.matches(&query, *query_node, TextProvider(&source));

            while let Some(query_match) = matches.next() {
                for cap in query_match.captures {
                    let node = cap.node;

                    let Some(highlight_name) = query.capture_names().get(cap.index as usize) else {
                        continue;
                    };

                    let node_range: Range<usize> = node.start_byte()..node.end_byte();
                    let highlight_name = SharedString::from(highlight_name.to_string());

                    // Merge near range and same highlight name
                    let last_item = highlights.last();
                    let last_range = last_item.map(|item| &item.range).unwrap_or(&(0..0));
                    let last_highlight_name = last_item.map(|item| item.name.clone());

                    if last_range == &node_range {
                        // case:
                        // last_range: 213..220, last_highlight_name: Some("property")
                        // last_range: 213..220, last_highlight_name: Some("string")
                        highlights.push(HighlightItem::new(
                            node_range,
                            last_highlight_name.unwrap_or(highlight_name),
                        ));
                    } else {
                        highlights.push(HighlightItem::new(node_range, highlight_name.clone()));
                    }
                }
            }
        }

        // DO NOT REMOVE THIS PRINT, it's useful for debugging
        // for item in highlights {
        //     println!("item: {:?}", item);
        // }

        highlights
    }

    /// Returns the syntax highlight styles for a range of text.
    ///
    /// The argument `range` is the range of bytes in the text to highlight.
    ///
    /// Returns a vector of tuples where each tuple contains:
    /// - A byte range relative to the text
    /// - The corresponding highlight style for that range
    ///
    /// # Example
    ///
    /// ```no_run
    /// use inazuma_component::highlighter::{HighlightTheme, SyntaxHighlighter};
    /// use ropey::Rope;
    ///
    /// let code = "fn main() {\n    println!(\"Hello\");\n}";
    /// let rope = Rope::from_str(code);
    /// let mut highlighter = SyntaxHighlighter::new("rust");
    /// highlighter.update(None, &rope, None);
    ///
    /// let theme = HighlightTheme::default_dark();
    /// let range = 0..code.len();
    /// let styles = highlighter.styles(&range, &theme);
    /// ```
    pub fn styles(
        &self,
        range: &Range<usize>,
        theme: &HighlightTheme,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        let mut styles = vec![];
        let start_offset = range.start;

        let highlights = self.match_styles(range.clone());

        // let mut iter_count = 0;
        for item in highlights {
            // iter_count += 1;
            let node_range = &item.range;
            let name = &item.name;

            // Avoid start larger than end
            let mut node_range = node_range.start.max(range.start)..node_range.end.min(range.end);
            if node_range.start > node_range.end {
                node_range.end = node_range.start;
            }

            styles.push((node_range, theme.style(name.as_ref()).unwrap_or_default()));
        }

        // If the matched styles is empty, return a default range.
        if styles.len() == 0 {
            return vec![(start_offset..range.end, HighlightStyle::default())];
        }

        let styles = unique_styles(&range, styles);

        // NOTE: DO NOT remove this comment, it is used for debugging.
        // for style in &styles {
        //     println!("---- style: {:?} - {:?}", style.0, style.1.color);
        // }
        // println!("--------------------------------");

        styles
    }
}

#[cfg(test)]
mod tests {
    use inazuma::Oklch;

    use super::*;
    use crate::Colorize as _;

    fn color_style(color: Oklch) -> HighlightStyle {
        let mut style = HighlightStyle::default();
        style.color = Some(color);
        style
    }

    #[track_caller]
    fn assert_unique_styles(
        range: Range<usize>,
        left: Vec<(Range<usize>, HighlightStyle)>,
        right: Vec<(Range<usize>, HighlightStyle)>,
    ) {
        fn color_name(c: Option<Oklch>) -> String {
            match c {
                Some(c) => {
                    if c == Oklch::from(inazuma::red()) {
                        "red".to_string()
                    } else if c == Oklch::from(inazuma::green()) {
                        "green".to_string()
                    } else if c == Oklch::from(inazuma::blue()) {
                        "blue".to_string()
                    } else {
                        c.to_hex()
                    }
                }
                None => "clean".to_string(),
            }
        }

        let left = unique_styles(&range, left);
        if left.len() != right.len() {
            println!("\n---------------------------------------------");
            for (range, style) in left.iter() {
                println!("({:?}, {})", range, color_name(style.color));
            }
            println!("---------------------------------------------");
            panic!("left {} styles, right {} styles", left.len(), right.len());
        }
        for (left, right) in left.into_iter().zip(right) {
            if left.1.color != right.1.color || left.0 != right.0 {
                panic!(
                    "\n left: ({:?}, {})\nright: ({:?}, {})\n",
                    left.0,
                    color_name(left.1.color),
                    right.0,
                    color_name(right.1.color)
                );
            }
        }
    }

    #[test]
    #[cfg(feature = "tree-sitter-languages")]
    fn test_php_combined_injection_closing_tags() {
        let php_code = r#"<?php
$x = 1;
?>
<html>
<body>
  <h1><?php echo "Hello"; ?></h1>
  <ul>
    <?php foreach ($items as $item): ?>
      <li><?php echo $item; ?></li>
    <?php endforeach; ?>
  </ul>
</body>
</html>
"#;

        let rope = Rope::from_str(php_code);
        let mut highlighter = SyntaxHighlighter::new("php");
        highlighter.update(None, &rope, None);

        assert!(
            highlighter.combined_injections_query.is_some(),
            "PHP should have combined injections query"
        );

        let full_range = 0..php_code.len();
        let highlights = highlighter.match_styles(full_range);

        // Verify all closing HTML tags are highlighted
        let closing_tags = ["</h1>", "</li>", "</ul>", "</body>", "</html>"];
        for tag in closing_tags {
            let pos = php_code.find(tag).unwrap();
            let tag_name_start = pos + 2; // after "</"
            let tag_name_end = tag_name_start + tag.len() - 3; // before ">"

            let has_highlight = highlights
                .iter()
                .any(|item| item.range.start <= tag_name_start && item.range.end >= tag_name_end);

            assert!(
                has_highlight,
                "closing tag {} at byte {} should be highlighted",
                tag, pos
            );
        }
    }

    #[test]
    fn test_unique_styles() {
        let red = color_style(Oklch::from(inazuma::red()));
        let green = color_style(Oklch::from(inazuma::green()));
        let blue = color_style(Oklch::from(inazuma::blue()));
        let clean = HighlightStyle::default();

        assert_unique_styles(
            0..65,
            vec![
                (2..10, clean),
                (2..10, clean),
                (5..11, red),
                (2..6, clean),
                (10..15, green),
                (15..30, clean),
                (29..35, blue),
                (35..40, green),
                (45..60, blue),
            ],
            vec![
                (0..5, clean),
                (5..6, red),
                (6..10, red),
                (10..11, green),
                (11..15, green),
                (15..29, clean),
                (29..30, blue),
                (30..35, blue),
                (35..40, green),
                (40..45, clean),
                (45..60, blue),
                (60..65, clean),
            ],
        );
    }
}
