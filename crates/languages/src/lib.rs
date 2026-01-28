//! # Languages Crate
//!
//! This crate manages Tree-sitter grammars and language configurations for the RustIDE.
//! It provides:
//!
//! - Language registry for managing available languages
//! - Grammar loading and configuration
//! - Syntax highlighting with theme support
//! - Language-specific configurations (LSP, DAP, comments, brackets)
//! - Built-in language definitions for common languages

mod builtin;
mod config;
mod grammar;
mod highlighter;
mod registry;

pub use builtin::{
    get_builtin_languages, get_c_language, get_go_language, get_javascript_language,
    get_json_language, get_python_language, get_rust_language, get_typescript_language,
    // TODO: Re-enable when tree-sitter-toml-ng and tree-sitter-md cc version conflict is resolved
    // get_toml_language, get_markdown_language,
};
pub use config::{
    AutoPair, BracketPair, CommentConfig, DapConfig, LanguageConfig, LspConfig, RunConfig,
};
pub use grammar::{Grammar, GrammarError, GrammarLoader, HighlightQuery};
pub use highlighter::{
    HighlightEvent, HighlightIterator, HighlightStyle, Highlighter, HighlighterError, Theme,
    ThemeStyle,
};
pub use registry::{LanguageDefinition, LanguageId, LanguageRegistry, RegistryError};

/// Initialize the language registry with all built-in languages.
///
/// This is the main entry point for setting up language support.
pub fn init_language_registry() -> LanguageRegistry {
    let mut registry = LanguageRegistry::new();

    for language in get_builtin_languages() {
        if let Err(e) = registry.register(language) {
            eprintln!("Failed to register built-in language: {}", e);
        }
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_registry() {
        let registry = init_language_registry();
        assert!(registry.get_by_id(&LanguageId::new("rust")).is_some());
        assert!(registry.get_by_extension("rs").is_some());
    }

    #[test]
    fn test_builtin_languages() {
        let languages = get_builtin_languages();
        assert!(!languages.is_empty());

        // Check that Rust is included
        assert!(languages.iter().any(|l| l.id.as_str() == "rust"));
    }
}
