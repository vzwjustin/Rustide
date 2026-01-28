//! Grammar Loading
//!
//! Handles loading and configuring Tree-sitter grammars for syntax parsing and highlighting.

use std::sync::Arc;
use thiserror::Error;
use tree_sitter::{Language, Parser, Query, QueryError};

/// Errors that can occur when working with grammars.
#[derive(Error, Debug)]
pub enum GrammarError {
    #[error("Failed to set language on parser: {0}")]
    LanguageSetError(String),

    #[error("Failed to compile query: {0}")]
    QueryCompileError(#[from] QueryError),

    #[error("Failed to parse: {0}")]
    ParseError(String),

    #[error("Grammar not found: {0}")]
    NotFound(String),

    #[error("Invalid highlight query: {0}")]
    InvalidHighlightQuery(String),
}

/// A highlight query for syntax highlighting.
#[derive(Debug, Clone)]
pub struct HighlightQuery {
    /// The raw query string.
    pub source: String,

    /// Compiled query for efficient matching.
    query: Arc<Query>,
}

impl HighlightQuery {
    /// Create a new highlight query from source.
    pub fn new(language: &Language, source: impl Into<String>) -> Result<Self, GrammarError> {
        let source = source.into();
        let query = Query::new(language, &source)?;
        Ok(Self {
            source,
            query: Arc::new(query),
        })
    }

    /// Get the compiled query.
    pub fn query(&self) -> &Query {
        &self.query
    }

    /// Get the capture names from this query.
    pub fn capture_names(&self) -> &[&str] {
        self.query.capture_names()
    }
}

/// A Tree-sitter grammar with associated queries.
#[derive(Debug, Clone)]
pub struct Grammar {
    /// The Tree-sitter language.
    language: Language,

    /// Highlight query for syntax highlighting.
    highlight_query: Option<HighlightQuery>,

    /// Injection query for embedded languages.
    injection_query: Option<HighlightQuery>,

    /// Locals query for local variable tracking.
    locals_query: Option<HighlightQuery>,
}

impl Grammar {
    /// Create a new grammar from a Tree-sitter language.
    pub fn new(language: Language) -> Self {
        Self {
            language,
            highlight_query: None,
            injection_query: None,
            locals_query: None,
        }
    }

    /// Get the Tree-sitter language.
    pub fn language(&self) -> &Language {
        &self.language
    }

    /// Set the highlight query.
    pub fn with_highlight_query(mut self, source: impl Into<String>) -> Result<Self, GrammarError> {
        self.highlight_query = Some(HighlightQuery::new(&self.language, source)?);
        Ok(self)
    }

    /// Set the injection query.
    pub fn with_injection_query(mut self, source: impl Into<String>) -> Result<Self, GrammarError> {
        self.injection_query = Some(HighlightQuery::new(&self.language, source)?);
        Ok(self)
    }

    /// Set the locals query.
    pub fn with_locals_query(mut self, source: impl Into<String>) -> Result<Self, GrammarError> {
        self.locals_query = Some(HighlightQuery::new(&self.language, source)?);
        Ok(self)
    }

    /// Get the highlight query.
    pub fn highlight_query(&self) -> Option<&HighlightQuery> {
        self.highlight_query.as_ref()
    }

    /// Get the injection query.
    pub fn injection_query(&self) -> Option<&HighlightQuery> {
        self.injection_query.as_ref()
    }

    /// Get the locals query.
    pub fn locals_query(&self) -> Option<&HighlightQuery> {
        self.locals_query.as_ref()
    }

    /// Create a new parser for this grammar.
    pub fn create_parser(&self) -> Result<Parser, GrammarError> {
        let mut parser = Parser::new();
        parser
            .set_language(&self.language)
            .map_err(|e| GrammarError::LanguageSetError(e.to_string()))?;
        Ok(parser)
    }

    /// Parse source code.
    pub fn parse(&self, source: &str, old_tree: Option<&tree_sitter::Tree>) -> Result<tree_sitter::Tree, GrammarError> {
        let mut parser = self.create_parser()?;
        parser
            .parse(source, old_tree)
            .ok_or_else(|| GrammarError::ParseError("Parser returned None".to_string()))
    }
}

/// Loader for Tree-sitter grammars.
pub struct GrammarLoader;

impl GrammarLoader {
    /// Load the Rust grammar with highlight queries.
    pub fn rust() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_rust::language())
            .with_highlight_query(RUST_HIGHLIGHTS)?;
        Ok(grammar)
    }

    /// Load the JavaScript grammar with highlight queries.
    pub fn javascript() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_javascript::language())
            .with_highlight_query(JAVASCRIPT_HIGHLIGHTS)?;
        Ok(grammar)
    }

    /// Load the Python grammar with highlight queries.
    pub fn python() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_python::language())
            .with_highlight_query(PYTHON_HIGHLIGHTS)?;
        Ok(grammar)
    }

    /// Load the C grammar with highlight queries.
    pub fn c() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_c::language())
            .with_highlight_query(C_HIGHLIGHTS)?;
        Ok(grammar)
    }

    /// Load the Go grammar with highlight queries.
    pub fn go() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_go::language())
            .with_highlight_query(GO_HIGHLIGHTS)?;
        Ok(grammar)
    }

    /// Load the JSON grammar with highlight queries.
    pub fn json() -> Result<Grammar, GrammarError> {
        let grammar = Grammar::new(tree_sitter_json::language())
            .with_highlight_query(JSON_HIGHLIGHTS)?;
        Ok(grammar)
    }

    // TODO: Re-enable TOML and Markdown grammars when cc version conflict is resolved
    // /// Load the TOML grammar with highlight queries.
    // pub fn toml() -> Result<Grammar, GrammarError> {
    //     let grammar = Grammar::new(tree_sitter_toml_ng::language())
    //         .with_highlight_query(TOML_HIGHLIGHTS)?;
    //     Ok(grammar)
    // }

    // /// Load the Markdown grammar with highlight queries.
    // pub fn markdown() -> Result<Grammar, GrammarError> {
    //     let grammar = Grammar::new(tree_sitter_md::language())
    //         .with_highlight_query(MARKDOWN_HIGHLIGHTS)?;
    //     Ok(grammar)
    // }
}

// Highlight queries for each language

/// Rust highlight queries
pub const RUST_HIGHLIGHTS: &str = r##"
; Keywords
"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"crate" @keyword
"dyn" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"false" @constant.builtin
"fn" @keyword.function
"for" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"macro_rules!" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"mut" @keyword
"pub" @keyword
"ref" @keyword
"return" @keyword
"self" @variable.builtin
"Self" @type.builtin
"static" @keyword
"struct" @keyword
"super" @variable.builtin
"trait" @keyword
"true" @constant.builtin
"type" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword

; Types
(type_identifier) @type
(primitive_type) @type.builtin
(generic_type (type_identifier) @type)

; Functions
(function_item name: (identifier) @function)
(function_signature_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))
(macro_invocation macro: (identifier) @function.macro)

; Variables
(identifier) @variable
(field_identifier) @property
(shorthand_field_initializer (identifier) @property)

; Parameters
(parameter pattern: (identifier) @variable.parameter)
(closure_parameters (identifier) @variable.parameter)

; Lifetimes
(lifetime (identifier) @label)

; Attributes
(attribute_item) @attribute
(inner_attribute_item) @attribute

; Literals
(string_literal) @string
(raw_string_literal) @string
(char_literal) @string
(escape_sequence) @string.escape
(integer_literal) @number
(float_literal) @number

; Comments
(line_comment) @comment
(block_comment) @comment

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"::" @punctuation.delimiter
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"&" @operator
"|" @operator
"^" @operator
"!" @operator
"=" @operator
"<" @operator
">" @operator
"==" @operator
"!=" @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
"->" @operator
"=>" @operator
".." @operator
"..=" @operator
"?" @operator
"#"##;

/// JavaScript highlight queries
pub const JAVASCRIPT_HIGHLIGHTS: &str = r##"
; Keywords
"async" @keyword
"await" @keyword
"break" @keyword
"case" @keyword
"catch" @keyword
"class" @keyword
"const" @keyword
"continue" @keyword
"debugger" @keyword
"default" @keyword
"delete" @keyword
"do" @keyword
"else" @keyword
"export" @keyword
"extends" @keyword
"finally" @keyword
"for" @keyword
"from" @keyword
"function" @keyword.function
"get" @keyword
"if" @keyword
"import" @keyword
"in" @keyword
"instanceof" @keyword
"let" @keyword
"new" @keyword
"of" @keyword
"return" @keyword
"set" @keyword
"static" @keyword
"switch" @keyword
"throw" @keyword
"try" @keyword
"typeof" @keyword
"var" @keyword
"void" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword

; Builtins
"this" @variable.builtin
"super" @variable.builtin
"true" @constant.builtin
"false" @constant.builtin
"null" @constant.builtin
"undefined" @constant.builtin

; Functions
(function_declaration name: (identifier) @function)
(function name: (identifier) @function)
(method_definition name: (property_identifier) @function.method)
(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (property_identifier) @function.method))
(arrow_function)

; Variables
(identifier) @variable
(property_identifier) @property
(shorthand_property_identifier) @property

; Parameters
(formal_parameters (identifier) @variable.parameter)

; Types (for TypeScript compatibility)
(type_identifier) @type

; Literals
(string) @string
(template_string) @string
(template_substitution) @punctuation.special
(regex) @string.regex
(number) @number

; Comments
(comment) @comment

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"**" @operator
"&" @operator
"|" @operator
"^" @operator
"~" @operator
"!" @operator
"=" @operator
"<" @operator
">" @operator
"==" @operator
"!=" @operator
"===" @operator
"!==" @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
"??" @operator
"++" @operator
"--" @operator
"=>" @operator
"?" @operator
"?." @operator
"..." @operator
"#"##;

/// Python highlight queries
pub const PYTHON_HIGHLIGHTS: &str = r##"
; Keywords
"and" @keyword
"as" @keyword
"assert" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"class" @keyword
"continue" @keyword
"def" @keyword.function
"del" @keyword
"elif" @keyword
"else" @keyword
"except" @keyword
"finally" @keyword
"for" @keyword
"from" @keyword
"global" @keyword
"if" @keyword
"import" @keyword
"in" @keyword
"is" @keyword
"lambda" @keyword
"nonlocal" @keyword
"not" @keyword
"or" @keyword
"pass" @keyword
"raise" @keyword
"return" @keyword
"try" @keyword
"while" @keyword
"with" @keyword
"yield" @keyword

; Builtins
"True" @constant.builtin
"False" @constant.builtin
"None" @constant.builtin
"self" @variable.builtin
"cls" @variable.builtin

; Functions
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
(call function: (attribute attribute: (identifier) @function.method))
(decorator (identifier) @function)

; Classes
(class_definition name: (identifier) @type)

; Variables
(identifier) @variable
(attribute attribute: (identifier) @property)

; Parameters
(parameters (identifier) @variable.parameter)
(default_parameter name: (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)
(keyword_argument name: (identifier) @variable.parameter)

; Literals
(string) @string
(interpolation) @punctuation.special
(integer) @number
(float) @number

; Comments
(comment) @comment

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"//" @operator
"%" @operator
"**" @operator
"&" @operator
"|" @operator
"^" @operator
"~" @operator
"=" @operator
"<" @operator
">" @operator
"==" @operator
"!=" @operator
"<=" @operator
">=" @operator
":=" @operator
"->" @operator
"@" @operator
"#"##;

/// C highlight queries
pub const C_HIGHLIGHTS: &str = r##"
; Keywords
"auto" @keyword
"break" @keyword
"case" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"do" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"for" @keyword
"goto" @keyword
"if" @keyword
"inline" @keyword
"register" @keyword
"return" @keyword
"sizeof" @keyword
"static" @keyword
"struct" @keyword
"switch" @keyword
"typedef" @keyword
"union" @keyword
"volatile" @keyword
"while" @keyword

; Types
(type_identifier) @type
(primitive_type) @type.builtin
(sized_type_specifier) @type.builtin

; Functions
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(call_expression function: (field_expression field: (field_identifier) @function.method))

; Variables
(identifier) @variable
(field_identifier) @property

; Parameters
(parameter_declaration declarator: (identifier) @variable.parameter)

; Preprocessor
(preproc_include) @keyword
(preproc_def) @keyword
(preproc_if) @keyword
(preproc_ifdef) @keyword
(preproc_else) @keyword
(preproc_elif) @keyword
(preproc_endif) @keyword
(preproc_directive) @keyword

; Macros
(preproc_function_def name: (identifier) @function.macro)
(preproc_call directive: (preproc_directive) @keyword argument: (preproc_arg) @string)

; Literals
(string_literal) @string
(char_literal) @string
(escape_sequence) @string.escape
(number_literal) @number
"true" @constant.builtin
"false" @constant.builtin
"NULL" @constant.builtin

; Comments
(comment) @comment

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter
"->" @punctuation.delimiter

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"&" @operator
"|" @operator
"^" @operator
"~" @operator
"!" @operator
"=" @operator
"<" @operator
">" @operator
"==" @operator
"!=" @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
"++" @operator
"--" @operator
"?" @operator
"#"##;

/// Go highlight queries
pub const GO_HIGHLIGHTS: &str = r##"
; Keywords
"break" @keyword
"case" @keyword
"chan" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"defer" @keyword
"else" @keyword
"fallthrough" @keyword
"for" @keyword
"func" @keyword.function
"go" @keyword
"goto" @keyword
"if" @keyword
"import" @keyword
"interface" @keyword
"map" @keyword
"package" @keyword
"range" @keyword
"return" @keyword
"select" @keyword
"struct" @keyword
"switch" @keyword
"type" @keyword
"var" @keyword

; Builtins
"true" @constant.builtin
"false" @constant.builtin
"nil" @constant.builtin
"iota" @constant.builtin

; Types
(type_identifier) @type
(type_spec name: (type_identifier) @type)
"int" @type.builtin
"int8" @type.builtin
"int16" @type.builtin
"int32" @type.builtin
"int64" @type.builtin
"uint" @type.builtin
"uint8" @type.builtin
"uint16" @type.builtin
"uint32" @type.builtin
"uint64" @type.builtin
"uintptr" @type.builtin
"float32" @type.builtin
"float64" @type.builtin
"complex64" @type.builtin
"complex128" @type.builtin
"bool" @type.builtin
"string" @type.builtin
"byte" @type.builtin
"rune" @type.builtin
"error" @type.builtin

; Functions
(function_declaration name: (identifier) @function)
(method_declaration name: (field_identifier) @function.method)
(call_expression function: (identifier) @function)
(call_expression function: (selector_expression field: (field_identifier) @function.method))

; Variables
(identifier) @variable
(field_identifier) @property

; Parameters
(parameter_declaration (identifier) @variable.parameter)

; Literals
(interpreted_string_literal) @string
(raw_string_literal) @string
(rune_literal) @string
(escape_sequence) @string.escape
(int_literal) @number
(float_literal) @number
(imaginary_literal) @number

; Comments
(comment) @comment

; Punctuation
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"&" @operator
"|" @operator
"^" @operator
"!" @operator
"=" @operator
"<" @operator
">" @operator
"==" @operator
"!=" @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
":=" @operator
"<-" @operator
"..." @operator
"#"##;

/// JSON highlight queries
pub const JSON_HIGHLIGHTS: &str = r##"
; Literals
(string) @string
(number) @number
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin

; Keys
(pair key: (string) @property)

; Punctuation
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
":" @punctuation.delimiter
"," @punctuation.delimiter
"#"##;

/// TOML highlight queries
pub const TOML_HIGHLIGHTS: &str = r##"
; Keys
(bare_key) @property
(quoted_key) @property
(dotted_key) @property

; Tables
(table (bare_key) @type)
(table (quoted_key) @type)
(table_array_element (bare_key) @type)
(table_array_element (quoted_key) @type)

; Literals
(string) @string
(integer) @number
(float) @number
(boolean) @constant.builtin
(offset_date_time) @string
(local_date_time) @string
(local_date) @string
(local_time) @string

; Comments
(comment) @comment

; Punctuation
"[" @punctuation.bracket
"]" @punctuation.bracket
"[[" @punctuation.bracket
"]]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"=" @punctuation.delimiter
"," @punctuation.delimiter
"." @punctuation.delimiter
"#"##;

/// Markdown highlight queries
pub const MARKDOWN_HIGHLIGHTS: &str = r##"
; Headings
(atx_heading (atx_h1_marker) @punctuation.special)
(atx_heading (atx_h2_marker) @punctuation.special)
(atx_heading (atx_h3_marker) @punctuation.special)
(atx_heading (atx_h4_marker) @punctuation.special)
(atx_heading (atx_h5_marker) @punctuation.special)
(atx_heading (atx_h6_marker) @punctuation.special)
(atx_heading (inline) @text.title)

; Lists
(list_marker_minus) @punctuation.special
(list_marker_plus) @punctuation.special
(list_marker_star) @punctuation.special
(list_marker_dot) @punctuation.special
(list_marker_parenthesis) @punctuation.special

; Code
(code_span) @text.literal
(fenced_code_block) @text.literal
(indented_code_block) @text.literal

; Links
(link_destination) @text.uri
(link_text) @text.reference
(link_label) @text.reference

; Emphasis
(emphasis) @text.emphasis
(strong_emphasis) @text.strong

; Block quotes
(block_quote) @text.quote

; Thematic breaks
(thematic_break) @punctuation.special
"#"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grammar_creation() {
        let grammar = Grammar::new(tree_sitter_rust::language());
        assert!(grammar.create_parser().is_ok());
    }

    #[test]
    fn test_grammar_with_highlight_query() {
        let grammar = GrammarLoader::rust().unwrap();
        assert!(grammar.highlight_query().is_some());
    }

    #[test]
    fn test_grammar_parse() {
        let grammar = GrammarLoader::rust().unwrap();
        let source = "fn main() { println!(\"Hello\"); }";
        let tree = grammar.parse(source, None).unwrap();
        assert_eq!(tree.root_node().kind(), "source_file");
    }

    #[test]
    fn test_all_grammars_load() {
        assert!(GrammarLoader::rust().is_ok());
        assert!(GrammarLoader::javascript().is_ok());
        assert!(GrammarLoader::python().is_ok());
        assert!(GrammarLoader::c().is_ok());
        assert!(GrammarLoader::go().is_ok());
        assert!(GrammarLoader::json().is_ok());
        // TODO: Re-enable when tree-sitter-toml-ng and tree-sitter-md cc version conflict is resolved
        // assert!(GrammarLoader::toml().is_ok());
        // assert!(GrammarLoader::markdown().is_ok());
    }

    #[test]
    fn test_highlight_query_capture_names() {
        let grammar = GrammarLoader::rust().unwrap();
        let query = grammar.highlight_query().unwrap();
        let names = query.capture_names();
        assert!(!names.is_empty());
    }
}
