//! Language Registry
//!
//! Manages the collection of available languages and provides lookup functionality.

use crate::config::LanguageConfig;
use crate::grammar::Grammar;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur when working with the language registry.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Language '{0}' is already registered")]
    AlreadyRegistered(String),

    #[error("Language '{0}' not found")]
    NotFound(String),

    #[error("Extension '{0}' is already registered to language '{1}'")]
    ExtensionConflict(String, String),

    #[error("Failed to load grammar for language '{0}': {1}")]
    GrammarLoadError(String, String),
}

/// A unique identifier for a language.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageId(String);

impl LanguageId {
    /// Create a new language ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for LanguageId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for LanguageId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Definition of a programming language.
///
/// Contains all information needed to provide IDE support for a language.
#[derive(Debug, Clone)]
pub struct LanguageDefinition {
    /// Unique identifier for the language (e.g., "rust", "python").
    pub id: LanguageId,

    /// Human-readable name (e.g., "Rust", "Python").
    pub display_name: String,

    /// File extensions associated with this language (without the dot).
    pub extensions: Vec<String>,

    /// File names that indicate this language (e.g., "Makefile", "Dockerfile").
    pub file_names: Vec<String>,

    /// The Tree-sitter grammar for parsing.
    pub grammar: Grammar,

    /// Language-specific configuration.
    pub config: LanguageConfig,

    /// MIME types associated with this language.
    pub mime_types: Vec<String>,

    /// First line patterns for detection (e.g., shebang lines).
    pub first_line_patterns: Vec<String>,
}

impl LanguageDefinition {
    /// Create a new language definition builder.
    pub fn builder(id: impl Into<String>, display_name: impl Into<String>) -> LanguageDefinitionBuilder {
        LanguageDefinitionBuilder::new(id, display_name)
    }

    /// Check if this language matches a file extension.
    pub fn matches_extension(&self, ext: &str) -> bool {
        let ext_lower = ext.to_lowercase();
        self.extensions.iter().any(|e| e.to_lowercase() == ext_lower)
    }

    /// Check if this language matches a file name.
    pub fn matches_filename(&self, name: &str) -> bool {
        self.file_names.iter().any(|n| n == name)
    }

    /// Check if this language matches a first line pattern.
    pub fn matches_first_line(&self, first_line: &str) -> bool {
        self.first_line_patterns.iter().any(|pattern| {
            first_line.contains(pattern)
        })
    }
}

/// Builder for creating LanguageDefinition instances.
pub struct LanguageDefinitionBuilder {
    id: LanguageId,
    display_name: String,
    extensions: Vec<String>,
    file_names: Vec<String>,
    grammar: Option<Grammar>,
    config: LanguageConfig,
    mime_types: Vec<String>,
    first_line_patterns: Vec<String>,
}

impl LanguageDefinitionBuilder {
    /// Create a new builder with required fields.
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: LanguageId::new(id),
            display_name: display_name.into(),
            extensions: Vec::new(),
            file_names: Vec::new(),
            grammar: None,
            config: LanguageConfig::default(),
            mime_types: Vec::new(),
            first_line_patterns: Vec::new(),
        }
    }

    /// Add file extensions for this language.
    pub fn extensions(mut self, extensions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.extensions = extensions.into_iter().map(Into::into).collect();
        self
    }

    /// Add file names for this language.
    pub fn file_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.file_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// Set the Tree-sitter grammar.
    pub fn grammar(mut self, grammar: Grammar) -> Self {
        self.grammar = Some(grammar);
        self
    }

    /// Set the language configuration.
    pub fn config(mut self, config: LanguageConfig) -> Self {
        self.config = config;
        self
    }

    /// Add MIME types.
    pub fn mime_types(mut self, types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.mime_types = types.into_iter().map(Into::into).collect();
        self
    }

    /// Add first line patterns for detection.
    pub fn first_line_patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.first_line_patterns = patterns.into_iter().map(Into::into).collect();
        self
    }

    /// Build the LanguageDefinition.
    ///
    /// # Panics
    ///
    /// Panics if no grammar was set.
    pub fn build(self) -> LanguageDefinition {
        LanguageDefinition {
            id: self.id,
            display_name: self.display_name,
            extensions: self.extensions,
            file_names: self.file_names,
            grammar: self.grammar.expect("Grammar is required"),
            config: self.config,
            mime_types: self.mime_types,
            first_line_patterns: self.first_line_patterns,
        }
    }

    /// Try to build the LanguageDefinition, returning an error if grammar is not set.
    pub fn try_build(self) -> Result<LanguageDefinition, RegistryError> {
        let grammar = self.grammar.ok_or_else(|| {
            RegistryError::GrammarLoadError(
                self.id.to_string(),
                "No grammar provided".to_string(),
            )
        })?;

        Ok(LanguageDefinition {
            id: self.id,
            display_name: self.display_name,
            extensions: self.extensions,
            file_names: self.file_names,
            grammar,
            config: self.config,
            mime_types: self.mime_types,
            first_line_patterns: self.first_line_patterns,
        })
    }
}

/// Registry of available programming languages.
///
/// Provides thread-safe access to language definitions and lookup by various criteria.
pub struct LanguageRegistry {
    /// Languages indexed by their ID.
    languages: RwLock<HashMap<LanguageId, Arc<LanguageDefinition>>>,

    /// Extension to language ID mapping for quick lookup.
    extension_map: RwLock<HashMap<String, LanguageId>>,

    /// File name to language ID mapping.
    filename_map: RwLock<HashMap<String, LanguageId>>,
}

impl LanguageRegistry {
    /// Create a new empty language registry.
    pub fn new() -> Self {
        Self {
            languages: RwLock::new(HashMap::new()),
            extension_map: RwLock::new(HashMap::new()),
            filename_map: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new language.
    pub fn register(&mut self, language: LanguageDefinition) -> Result<(), RegistryError> {
        let id = language.id.clone();

        // Check if already registered
        if self.languages.read().contains_key(&id) {
            return Err(RegistryError::AlreadyRegistered(id.to_string()));
        }

        // Check for extension conflicts
        {
            let ext_map = self.extension_map.read();
            for ext in &language.extensions {
                if let Some(existing_id) = ext_map.get(ext) {
                    return Err(RegistryError::ExtensionConflict(
                        ext.clone(),
                        existing_id.to_string(),
                    ));
                }
            }
        }

        // Register extensions
        {
            let mut ext_map = self.extension_map.write();
            for ext in &language.extensions {
                ext_map.insert(ext.to_lowercase(), id.clone());
            }
        }

        // Register file names
        {
            let mut name_map = self.filename_map.write();
            for name in &language.file_names {
                name_map.insert(name.clone(), id.clone());
            }
        }

        // Store the language
        self.languages.write().insert(id, Arc::new(language));

        Ok(())
    }

    /// Get a language by its ID.
    pub fn get_by_id(&self, id: &LanguageId) -> Option<Arc<LanguageDefinition>> {
        self.languages.read().get(id).cloned()
    }

    /// Get a language by file extension.
    pub fn get_by_extension(&self, ext: &str) -> Option<Arc<LanguageDefinition>> {
        let ext_lower = ext.to_lowercase();
        let id = self.extension_map.read().get(&ext_lower).cloned()?;
        self.get_by_id(&id)
    }

    /// Get a language by file name.
    pub fn get_by_filename(&self, name: &str) -> Option<Arc<LanguageDefinition>> {
        let id = self.filename_map.read().get(name).cloned()?;
        self.get_by_id(&id)
    }

    /// Detect the language for a file based on path and optional first line.
    pub fn detect_language(
        &self,
        path: &std::path::Path,
        first_line: Option<&str>,
    ) -> Option<Arc<LanguageDefinition>> {
        // First, try by file name (for files like Makefile, Dockerfile)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if let Some(lang) = self.get_by_filename(name) {
                return Some(lang);
            }
        }

        // Then try by extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(lang) = self.get_by_extension(ext) {
                return Some(lang);
            }
        }

        // Finally, try first line patterns (shebangs, etc.)
        if let Some(first_line) = first_line {
            for lang in self.languages.read().values() {
                if lang.matches_first_line(first_line) {
                    return Some(lang.clone());
                }
            }
        }

        None
    }

    /// Get all registered languages.
    pub fn all_languages(&self) -> Vec<Arc<LanguageDefinition>> {
        self.languages.read().values().cloned().collect()
    }

    /// Get the number of registered languages.
    pub fn len(&self) -> usize {
        self.languages.read().len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.languages.read().is_empty()
    }

    /// Unregister a language by ID.
    pub fn unregister(&mut self, id: &LanguageId) -> Option<Arc<LanguageDefinition>> {
        let lang = self.languages.write().remove(id)?;

        // Remove extension mappings
        {
            let mut ext_map = self.extension_map.write();
            for ext in &lang.extensions {
                ext_map.remove(&ext.to_lowercase());
            }
        }

        // Remove filename mappings
        {
            let mut name_map = self.filename_map.write();
            for name in &lang.file_names {
                name_map.remove(name);
            }
        }

        Some(lang)
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::Grammar;

    fn create_test_grammar() -> Grammar {
        Grammar::new(tree_sitter_rust::LANGUAGE.into())
    }

    #[test]
    fn test_language_id() {
        let id = LanguageId::new("rust");
        assert_eq!(id.as_str(), "rust");
        assert_eq!(id.to_string(), "rust");
    }

    #[test]
    fn test_language_definition_builder() {
        let lang = LanguageDefinition::builder("rust", "Rust")
            .extensions(["rs"])
            .grammar(create_test_grammar())
            .build();

        assert_eq!(lang.id.as_str(), "rust");
        assert_eq!(lang.display_name, "Rust");
        assert!(lang.matches_extension("rs"));
    }

    #[test]
    fn test_registry_register_and_lookup() {
        let mut registry = LanguageRegistry::new();

        let rust = LanguageDefinition::builder("rust", "Rust")
            .extensions(["rs"])
            .grammar(create_test_grammar())
            .build();

        registry.register(rust).unwrap();

        assert!(registry.get_by_id(&LanguageId::new("rust")).is_some());
        assert!(registry.get_by_extension("rs").is_some());
        assert!(registry.get_by_extension("RS").is_some()); // Case insensitive
    }

    #[test]
    fn test_registry_duplicate_language() {
        let mut registry = LanguageRegistry::new();

        let rust1 = LanguageDefinition::builder("rust", "Rust")
            .extensions(["rs"])
            .grammar(create_test_grammar())
            .build();

        let rust2 = LanguageDefinition::builder("rust", "Rust 2")
            .extensions(["rs2"])
            .grammar(create_test_grammar())
            .build();

        registry.register(rust1).unwrap();
        let result = registry.register(rust2);
        assert!(matches!(result, Err(RegistryError::AlreadyRegistered(_))));
    }

    #[test]
    fn test_detect_language() {
        let mut registry = LanguageRegistry::new();

        let rust = LanguageDefinition::builder("rust", "Rust")
            .extensions(["rs"])
            .grammar(create_test_grammar())
            .build();

        registry.register(rust).unwrap();

        let path = std::path::Path::new("/path/to/file.rs");
        let detected = registry.detect_language(path, None);
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().id.as_str(), "rust");
    }
}
