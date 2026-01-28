//! Syntax Highlighting
//!
//! Provides theme-aware syntax highlighting using Tree-sitter.

use crate::grammar::Grammar;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use thiserror::Error;
use tree_sitter::{Parser, QueryCursor, Tree};

/// Errors that can occur during highlighting.
#[derive(Error, Debug)]
pub enum HighlighterError {
    #[error("Failed to parse source code")]
    ParseError,

    #[error("No highlight query available for this language")]
    NoHighlightQuery,

    #[error("Failed to create parser: {0}")]
    ParserError(String),

    #[error("Invalid range: {0}")]
    InvalidRange(String),
}

/// A style applied to highlighted text.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightStyle {
    /// The highlight group name (e.g., "keyword", "function", "string").
    pub group: String,

    /// Priority for overlapping highlights (higher wins).
    pub priority: u32,
}

impl HighlightStyle {
    /// Create a new highlight style.
    pub fn new(group: impl Into<String>) -> Self {
        Self {
            group: group.into(),
            priority: 0,
        }
    }

    /// Create a highlight style with priority.
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// A theme style definition.
#[derive(Debug, Clone, Default)]
pub struct ThemeStyle {
    /// Foreground color as RGBA hex (e.g., 0xFF0000FF for red).
    pub foreground: Option<u32>,

    /// Background color as RGBA hex.
    pub background: Option<u32>,

    /// Whether text is bold.
    pub bold: bool,

    /// Whether text is italic.
    pub italic: bool,

    /// Whether text has underline.
    pub underline: bool,

    /// Whether text has strikethrough.
    pub strikethrough: bool,
}

impl ThemeStyle {
    /// Create a new theme style with foreground color.
    pub fn foreground(color: u32) -> Self {
        Self {
            foreground: Some(color),
            ..Default::default()
        }
    }

    /// Set the foreground color.
    pub fn with_foreground(mut self, color: u32) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Set the background color.
    pub fn with_background(mut self, color: u32) -> Self {
        self.background = Some(color);
        self
    }

    /// Set bold style.
    pub fn with_bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set italic style.
    pub fn with_italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Set underline style.
    pub fn with_underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Set strikethrough style.
    pub fn with_strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }
}

/// A syntax highlighting theme.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name.
    pub name: String,

    /// Style mappings from highlight group to theme style.
    styles: HashMap<String, ThemeStyle>,

    /// Default style for unhighlighted text.
    pub default_style: ThemeStyle,
}

impl Theme {
    /// Create a new theme.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            styles: HashMap::new(),
            default_style: ThemeStyle::default(),
        }
    }

    /// Add a style for a highlight group.
    pub fn add_style(&mut self, group: impl Into<String>, style: ThemeStyle) {
        self.styles.insert(group.into(), style);
    }

    /// Get the style for a highlight group.
    pub fn get_style(&self, group: &str) -> Option<&ThemeStyle> {
        // Try exact match first
        if let Some(style) = self.styles.get(group) {
            return Some(style);
        }

        // Try parent groups (e.g., "function.method" -> "function")
        let mut current = group;
        while let Some(dot_pos) = current.rfind('.') {
            current = &current[..dot_pos];
            if let Some(style) = self.styles.get(current) {
                return Some(style);
            }
        }

        None
    }

    /// Get or create a default dark theme.
    pub fn dark() -> Self {
        let mut theme = Theme::new("Dark");

        // Set default style
        theme.default_style = ThemeStyle::foreground(0xD4D4D4FF);

        // Keywords
        theme.add_style("keyword", ThemeStyle::foreground(0xC586C0FF));
        theme.add_style("keyword.function", ThemeStyle::foreground(0x569CD6FF));

        // Types
        theme.add_style("type", ThemeStyle::foreground(0x4EC9B0FF));
        theme.add_style("type.builtin", ThemeStyle::foreground(0x4EC9B0FF));

        // Functions
        theme.add_style("function", ThemeStyle::foreground(0xDCDCABFF));
        theme.add_style("function.method", ThemeStyle::foreground(0xDCDCABFF));
        theme.add_style("function.macro", ThemeStyle::foreground(0x569CD6FF));

        // Variables
        theme.add_style("variable", ThemeStyle::foreground(0x9CDCFEFF));
        theme.add_style("variable.parameter", ThemeStyle::foreground(0x9CDCFEFF));
        theme.add_style("variable.builtin", ThemeStyle::foreground(0x569CD6FF));

        // Properties
        theme.add_style("property", ThemeStyle::foreground(0x9CDCFEFF));

        // Constants
        theme.add_style("constant", ThemeStyle::foreground(0x4FC1FFFF));
        theme.add_style("constant.builtin", ThemeStyle::foreground(0x569CD6FF));

        // Strings
        theme.add_style("string", ThemeStyle::foreground(0xCE9178FF));
        theme.add_style("string.escape", ThemeStyle::foreground(0xD7BA7DFF));
        theme.add_style("string.regex", ThemeStyle::foreground(0xD16969FF));

        // Numbers
        theme.add_style("number", ThemeStyle::foreground(0xB5CEA8FF));

        // Comments
        theme.add_style("comment", ThemeStyle::foreground(0x6A9955FF).with_italic());

        // Operators
        theme.add_style("operator", ThemeStyle::foreground(0xD4D4D4FF));

        // Punctuation
        theme.add_style("punctuation", ThemeStyle::foreground(0xD4D4D4FF));
        theme.add_style("punctuation.bracket", ThemeStyle::foreground(0xFFD700FF));
        theme.add_style("punctuation.delimiter", ThemeStyle::foreground(0xD4D4D4FF));
        theme.add_style("punctuation.special", ThemeStyle::foreground(0x569CD6FF));

        // Labels
        theme.add_style("label", ThemeStyle::foreground(0xC586C0FF));

        // Attributes
        theme.add_style("attribute", ThemeStyle::foreground(0x4EC9B0FF));

        // Text (for Markdown, etc.)
        theme.add_style("text", ThemeStyle::foreground(0xD4D4D4FF));
        theme.add_style("text.title", ThemeStyle::foreground(0x569CD6FF).with_bold());
        theme.add_style("text.literal", ThemeStyle::foreground(0xCE9178FF));
        theme.add_style("text.uri", ThemeStyle::foreground(0x4EC9B0FF).with_underline());
        theme.add_style("text.reference", ThemeStyle::foreground(0x9CDCFEFF));
        theme.add_style("text.emphasis", ThemeStyle::foreground(0xD4D4D4FF).with_italic());
        theme.add_style("text.strong", ThemeStyle::foreground(0xD4D4D4FF).with_bold());
        theme.add_style("text.quote", ThemeStyle::foreground(0x6A9955FF).with_italic());

        theme
    }

    /// Get or create a default light theme.
    pub fn light() -> Self {
        let mut theme = Theme::new("Light");

        // Set default style
        theme.default_style = ThemeStyle::foreground(0x000000FF);

        // Keywords
        theme.add_style("keyword", ThemeStyle::foreground(0xAF00DBFF));
        theme.add_style("keyword.function", ThemeStyle::foreground(0x0000FFFF));

        // Types
        theme.add_style("type", ThemeStyle::foreground(0x267F99FF));
        theme.add_style("type.builtin", ThemeStyle::foreground(0x0000FFFF));

        // Functions
        theme.add_style("function", ThemeStyle::foreground(0x795E26FF));
        theme.add_style("function.method", ThemeStyle::foreground(0x795E26FF));
        theme.add_style("function.macro", ThemeStyle::foreground(0x0000FFFF));

        // Variables
        theme.add_style("variable", ThemeStyle::foreground(0x001080FF));
        theme.add_style("variable.parameter", ThemeStyle::foreground(0x001080FF));
        theme.add_style("variable.builtin", ThemeStyle::foreground(0x0000FFFF));

        // Properties
        theme.add_style("property", ThemeStyle::foreground(0x001080FF));

        // Constants
        theme.add_style("constant", ThemeStyle::foreground(0x0070C1FF));
        theme.add_style("constant.builtin", ThemeStyle::foreground(0x0000FFFF));

        // Strings
        theme.add_style("string", ThemeStyle::foreground(0xA31515FF));
        theme.add_style("string.escape", ThemeStyle::foreground(0xEE0000FF));
        theme.add_style("string.regex", ThemeStyle::foreground(0x811F3FFF));

        // Numbers
        theme.add_style("number", ThemeStyle::foreground(0x098658FF));

        // Comments
        theme.add_style("comment", ThemeStyle::foreground(0x008000FF).with_italic());

        // Operators
        theme.add_style("operator", ThemeStyle::foreground(0x000000FF));

        // Punctuation
        theme.add_style("punctuation", ThemeStyle::foreground(0x000000FF));
        theme.add_style("punctuation.bracket", ThemeStyle::foreground(0x0431FAFF));
        theme.add_style("punctuation.delimiter", ThemeStyle::foreground(0x000000FF));
        theme.add_style("punctuation.special", ThemeStyle::foreground(0x0000FFFF));

        // Labels
        theme.add_style("label", ThemeStyle::foreground(0xAF00DBFF));

        // Attributes
        theme.add_style("attribute", ThemeStyle::foreground(0x267F99FF));

        // Text
        theme.add_style("text", ThemeStyle::foreground(0x000000FF));
        theme.add_style("text.title", ThemeStyle::foreground(0x0000FFFF).with_bold());
        theme.add_style("text.literal", ThemeStyle::foreground(0xA31515FF));
        theme.add_style("text.uri", ThemeStyle::foreground(0x267F99FF).with_underline());
        theme.add_style("text.reference", ThemeStyle::foreground(0x001080FF));
        theme.add_style("text.emphasis", ThemeStyle::foreground(0x000000FF).with_italic());
        theme.add_style("text.strong", ThemeStyle::foreground(0x000000FF).with_bold());
        theme.add_style("text.quote", ThemeStyle::foreground(0x008000FF).with_italic());

        theme
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// An event in the highlight stream.
#[derive(Debug, Clone, PartialEq)]
pub enum HighlightEvent {
    /// Start of a highlighted region with the given style.
    Start(HighlightStyle),

    /// End of the current highlighted region.
    End,

    /// Source text that should be emitted.
    Source {
        /// Byte range in the source.
        range: Range<usize>,
    },
}

/// Iterator over highlight events for incremental rendering.
pub struct HighlightIterator<'a> {
    source: &'a str,
    events: Vec<HighlightEvent>,
    index: usize,
}

impl<'a> HighlightIterator<'a> {
    /// Create a new highlight iterator.
    pub fn new(source: &'a str, events: Vec<HighlightEvent>) -> Self {
        Self {
            source,
            events,
            index: 0,
        }
    }

    /// Get the source text.
    pub fn source(&self) -> &str {
        self.source
    }
}

impl<'a> Iterator for HighlightIterator<'a> {
    type Item = HighlightEvent;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.events.len() {
            let event = self.events[self.index].clone();
            self.index += 1;
            Some(event)
        } else {
            None
        }
    }
}

/// Syntax highlighter with incremental support.
pub struct Highlighter {
    /// The grammar for this highlighter.
    grammar: Grammar,

    /// Reusable parser.
    parser: RwLock<Parser>,

    /// The current parse tree.
    tree: RwLock<Option<Tree>>,

    /// Capture name to highlight group mapping.
    capture_map: HashMap<String, String>,
}

impl Highlighter {
    /// Create a new highlighter for a grammar.
    pub fn new(grammar: Grammar) -> Result<Self, HighlighterError> {
        let parser = grammar
            .create_parser()
            .map_err(|e| HighlighterError::ParserError(e.to_string()))?;

        // Build capture name mapping
        let mut capture_map = HashMap::new();
        if let Some(query) = grammar.highlight_query() {
            for name in query.capture_names() {
                // Map tree-sitter capture names to our highlight groups
                capture_map.insert(name.to_string(), name.to_string());
            }
        }

        Ok(Self {
            grammar,
            parser: RwLock::new(parser),
            tree: RwLock::new(None),
            capture_map,
        })
    }

    /// Parse the source code and update the internal tree.
    pub fn parse(&self, source: &str) -> Result<(), HighlighterError> {
        let old_tree = self.tree.read().as_ref().cloned();
        let mut parser = self.parser.write();

        let tree = parser
            .parse(source, old_tree.as_ref())
            .ok_or(HighlighterError::ParseError)?;

        *self.tree.write() = Some(tree);
        Ok(())
    }

    /// Update the tree after an edit.
    ///
    /// This enables incremental parsing for better performance.
    pub fn edit(
        &self,
        edit: &tree_sitter::InputEdit,
        new_source: &str,
    ) -> Result<(), HighlighterError> {
        // Apply the edit to the existing tree
        if let Some(ref mut tree) = *self.tree.write() {
            tree.edit(edit);
        }

        // Re-parse with the edited tree
        self.parse(new_source)
    }

    /// Generate highlight events for a range of the source.
    pub fn highlight<'a>(
        &self,
        source: &'a str,
        range: Option<Range<usize>>,
    ) -> Result<HighlightIterator<'a>, HighlighterError> {
        let query = self
            .grammar
            .highlight_query()
            .ok_or(HighlighterError::NoHighlightQuery)?;

        let tree = self.tree.read();
        let tree = tree.as_ref().ok_or(HighlighterError::ParseError)?;

        let range = range.unwrap_or(0..source.len());

        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(range.clone());

        let mut matches = cursor.matches(query.query(), tree.root_node(), source.as_bytes());

        // Collect all highlights with their ranges
        let mut highlights: Vec<(Range<usize>, String)> = Vec::new();

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();

                if start >= range.start && end <= range.end {
                    let capture_name = query.capture_names()[capture.index as usize];
                    if let Some(group) = self.capture_map.get(capture_name) {
                        highlights.push((start..end, group.clone()));
                    }
                }
            }
        }

        // Sort by start position, then by length (longer matches first for nesting)
        highlights.sort_by(|a, b| {
            a.0.start
                .cmp(&b.0.start)
                .then_with(|| b.0.len().cmp(&a.0.len()))
        });

        // Generate events
        let mut events = Vec::new();
        let mut pos = range.start;
        let mut active_styles: Vec<(usize, String)> = Vec::new(); // (end_pos, group)

        for (highlight_range, group) in highlights {
            // Close any styles that end before this highlight starts
            while let Some((end, _)) = active_styles.last() {
                if *end <= highlight_range.start {
                    events.push(HighlightEvent::End);
                    active_styles.pop();
                } else {
                    break;
                }
            }

            // Emit source text before this highlight
            if highlight_range.start > pos {
                events.push(HighlightEvent::Source {
                    range: pos..highlight_range.start,
                });
            }

            // Start the new highlight
            events.push(HighlightEvent::Start(HighlightStyle::new(&group)));
            active_styles.push((highlight_range.end, group));

            pos = highlight_range.start;
        }

        // Close remaining styles and emit remaining source
        while active_styles.pop().is_some() {
            events.push(HighlightEvent::End);
        }

        if pos < range.end {
            events.push(HighlightEvent::Source { range: pos..range.end });
        }

        Ok(HighlightIterator::new(source, events))
    }

    /// Highlight source code and return styled spans.
    ///
    /// This is a convenience method that returns a vector of (range, style) pairs.
    pub fn highlight_spans(
        &self,
        source: &str,
    ) -> Result<Vec<(Range<usize>, Option<String>)>, HighlighterError> {
        // Ensure we have a parse tree
        if self.tree.read().is_none() {
            self.parse(source)?;
        }

        let query = self
            .grammar
            .highlight_query()
            .ok_or(HighlighterError::NoHighlightQuery)?;

        let tree = self.tree.read();
        let tree = tree.as_ref().ok_or(HighlighterError::ParseError)?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query.query(), tree.root_node(), source.as_bytes());

        let mut spans: Vec<(Range<usize>, Option<String>)> = Vec::new();
        let mut covered = vec![false; source.len()];

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();
                let capture_name = query.capture_names()[capture.index as usize];

                if let Some(group) = self.capture_map.get(capture_name) {
                    // Mark bytes as covered
                    for i in start..end.min(source.len()) {
                        covered[i] = true;
                    }
                    spans.push((start..end, Some(group.clone())));
                }
            }
        }

        // Add unhighlighted regions
        let mut unhighlighted_start = None;
        for (i, &is_covered) in covered.iter().enumerate() {
            if !is_covered {
                if unhighlighted_start.is_none() {
                    unhighlighted_start = Some(i);
                }
            } else if let Some(start) = unhighlighted_start {
                spans.push((start..i, None));
                unhighlighted_start = None;
            }
        }

        if let Some(start) = unhighlighted_start {
            spans.push((start..source.len(), None));
        }

        // Sort by start position
        spans.sort_by_key(|(range, _)| range.start);

        Ok(spans)
    }

    /// Get the current parse tree.
    pub fn tree(&self) -> Option<Tree> {
        self.tree.read().clone()
    }

    /// Get the grammar.
    pub fn grammar(&self) -> &Grammar {
        &self.grammar
    }
}

/// Create a highlighter for a language.
pub fn create_highlighter(grammar: Grammar) -> Result<Arc<Highlighter>, HighlighterError> {
    Ok(Arc::new(Highlighter::new(grammar)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::GrammarLoader;

    #[test]
    fn test_highlighter_creation() {
        let grammar = GrammarLoader::rust().unwrap();
        let highlighter = Highlighter::new(grammar);
        assert!(highlighter.is_ok());
    }

    #[test]
    fn test_parse_and_highlight() {
        let grammar = GrammarLoader::rust().unwrap();
        let highlighter = Highlighter::new(grammar).unwrap();

        let source = "fn main() {}";
        highlighter.parse(source).unwrap();

        let spans = highlighter.highlight_spans(source).unwrap();
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_theme_dark() {
        let theme = Theme::dark();
        assert!(theme.get_style("keyword").is_some());
        assert!(theme.get_style("function").is_some());
        assert!(theme.get_style("string").is_some());
    }

    #[test]
    fn test_theme_light() {
        let theme = Theme::light();
        assert!(theme.get_style("keyword").is_some());
        assert!(theme.get_style("function").is_some());
        assert!(theme.get_style("string").is_some());
    }

    #[test]
    fn test_theme_parent_group_fallback() {
        let mut theme = Theme::new("Test");
        theme.add_style("function", ThemeStyle::foreground(0xFF0000FF));

        // Should fall back to "function" for "function.method"
        assert!(theme.get_style("function.method").is_some());
    }

    #[test]
    fn test_highlight_rust_code() {
        let grammar = GrammarLoader::rust().unwrap();
        let highlighter = Highlighter::new(grammar).unwrap();

        let source = r#"
fn main() {
    let x = 42;
    println!("Hello, world!");
}
"#;

        highlighter.parse(source).unwrap();
        let spans = highlighter.highlight_spans(source).unwrap();

        // Should have highlights for keywords, functions, numbers, strings
        let groups: Vec<_> = spans.iter().filter_map(|(_, g)| g.clone()).collect();
        assert!(groups.iter().any(|g| g.contains("keyword")));
        assert!(groups.iter().any(|g| g.contains("function")));
    }

    #[test]
    fn test_highlight_iterator() {
        let grammar = GrammarLoader::rust().unwrap();
        let highlighter = Highlighter::new(grammar).unwrap();

        let source = "let x = 5;";
        highlighter.parse(source).unwrap();

        let iter = highlighter.highlight(source, None).unwrap();
        let events: Vec<_> = iter.collect();
        assert!(!events.is_empty());
    }
}
