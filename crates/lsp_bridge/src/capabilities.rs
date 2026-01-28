//! Capability handling for LSP servers.
//!
//! This module provides utilities for parsing and querying server capabilities,
//! enabling feature detection for different language servers.

use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, HoverProviderCapability,
    OneOf, ServerCapabilities, SignatureHelpOptions, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions,
};

/// Parsed server capabilities with convenient accessors.
#[derive(Debug, Clone, Default)]
pub struct ServerCapabilityInfo {
    /// Raw capabilities from the server.
    pub raw: ServerCapabilities,
}

impl ServerCapabilityInfo {
    /// Create a new capability info from raw capabilities.
    pub fn new(capabilities: ServerCapabilities) -> Self {
        Self { raw: capabilities }
    }

    /// Check if the server supports hover.
    pub fn supports_hover(&self) -> bool {
        match &self.raw.hover_provider {
            Some(HoverProviderCapability::Simple(true)) => true,
            Some(HoverProviderCapability::Options(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports completion.
    pub fn supports_completion(&self) -> bool {
        self.raw.completion_provider.is_some()
    }

    /// Get completion options if available.
    pub fn completion_options(&self) -> Option<&CompletionOptions> {
        self.raw.completion_provider.as_ref()
    }

    /// Get completion trigger characters.
    pub fn completion_trigger_characters(&self) -> Vec<&str> {
        self.raw
            .completion_provider
            .as_ref()
            .and_then(|opts| opts.trigger_characters.as_ref())
            .map(|chars| chars.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Check if the server supports signature help.
    pub fn supports_signature_help(&self) -> bool {
        self.raw.signature_help_provider.is_some()
    }

    /// Get signature help options if available.
    pub fn signature_help_options(&self) -> Option<&SignatureHelpOptions> {
        self.raw.signature_help_provider.as_ref()
    }

    /// Get signature help trigger characters.
    pub fn signature_help_trigger_characters(&self) -> Vec<&str> {
        self.raw
            .signature_help_provider
            .as_ref()
            .and_then(|opts| opts.trigger_characters.as_ref())
            .map(|chars| chars.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Check if the server supports go to definition.
    pub fn supports_definition(&self) -> bool {
        match &self.raw.definition_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports go to type definition.
    pub fn supports_type_definition(&self) -> bool {
        self.raw.type_definition_provider.is_some()
    }

    /// Check if the server supports go to implementation.
    pub fn supports_implementation(&self) -> bool {
        self.raw.implementation_provider.is_some()
    }

    /// Check if the server supports find references.
    pub fn supports_references(&self) -> bool {
        match &self.raw.references_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports document highlights.
    pub fn supports_document_highlight(&self) -> bool {
        match &self.raw.document_highlight_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports document symbols.
    pub fn supports_document_symbol(&self) -> bool {
        match &self.raw.document_symbol_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports workspace symbols.
    pub fn supports_workspace_symbol(&self) -> bool {
        match &self.raw.workspace_symbol_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports code actions.
    pub fn supports_code_action(&self) -> bool {
        match &self.raw.code_action_provider {
            Some(CodeActionProviderCapability::Simple(true)) => true,
            Some(CodeActionProviderCapability::Options(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports code lens.
    pub fn supports_code_lens(&self) -> bool {
        self.raw.code_lens_provider.is_some()
    }

    /// Check if the server supports document formatting.
    pub fn supports_formatting(&self) -> bool {
        match &self.raw.document_formatting_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports range formatting.
    pub fn supports_range_formatting(&self) -> bool {
        match &self.raw.document_range_formatting_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports on-type formatting.
    pub fn supports_on_type_formatting(&self) -> bool {
        self.raw.document_on_type_formatting_provider.is_some()
    }

    /// Get on-type formatting trigger characters.
    pub fn on_type_formatting_trigger_characters(&self) -> Vec<&str> {
        self.raw
            .document_on_type_formatting_provider
            .as_ref()
            .map(|opts| {
                let mut chars = vec![opts.first_trigger_character.as_str()];
                if let Some(more) = &opts.more_trigger_character {
                    chars.extend(more.iter().map(|s| s.as_str()));
                }
                chars
            })
            .unwrap_or_default()
    }

    /// Check if the server supports rename.
    pub fn supports_rename(&self) -> bool {
        match &self.raw.rename_provider {
            Some(OneOf::Left(true)) => true,
            Some(OneOf::Right(_)) => true,
            _ => false,
        }
    }

    /// Check if the server supports prepare rename.
    pub fn supports_prepare_rename(&self) -> bool {
        match &self.raw.rename_provider {
            Some(OneOf::Right(opts)) => opts.prepare_provider.unwrap_or(false),
            _ => false,
        }
    }

    /// Check if the server supports folding ranges.
    pub fn supports_folding_range(&self) -> bool {
        self.raw.folding_range_provider.is_some()
    }

    /// Check if the server supports selection ranges.
    pub fn supports_selection_range(&self) -> bool {
        self.raw.selection_range_provider.is_some()
    }

    /// Check if the server supports semantic tokens.
    pub fn supports_semantic_tokens(&self) -> bool {
        self.raw.semantic_tokens_provider.is_some()
    }

    /// Check if the server supports inlay hints.
    pub fn supports_inlay_hints(&self) -> bool {
        self.raw.inlay_hint_provider.is_some()
    }

    /// Check if the server supports call hierarchy.
    pub fn supports_call_hierarchy(&self) -> bool {
        self.raw.call_hierarchy_provider.is_some()
    }

    /// Get the text document sync kind.
    pub fn text_document_sync_kind(&self) -> TextDocumentSyncKind {
        match &self.raw.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => *kind,
            Some(TextDocumentSyncCapability::Options(opts)) => {
                opts.change.unwrap_or(TextDocumentSyncKind::NONE)
            }
            None => TextDocumentSyncKind::NONE,
        }
    }

    /// Get text document sync options if available.
    pub fn text_document_sync_options(&self) -> Option<TextDocumentSyncOptions> {
        match &self.raw.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => Some(TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(*kind),
                will_save: None,
                will_save_wait_until: None,
                save: None,
            }),
            Some(TextDocumentSyncCapability::Options(opts)) => Some(opts.clone()),
            None => None,
        }
    }

    /// Check if the server wants open/close notifications.
    pub fn wants_open_close(&self) -> bool {
        match &self.raw.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(_)) => true,
            Some(TextDocumentSyncCapability::Options(opts)) => opts.open_close.unwrap_or(false),
            None => false,
        }
    }

    /// Check if the server supports will save notifications.
    pub fn supports_will_save(&self) -> bool {
        match &self.raw.text_document_sync {
            Some(TextDocumentSyncCapability::Options(opts)) => opts.will_save.unwrap_or(false),
            _ => false,
        }
    }

    /// Check if the server supports will save wait until.
    pub fn supports_will_save_wait_until(&self) -> bool {
        match &self.raw.text_document_sync {
            Some(TextDocumentSyncCapability::Options(opts)) => {
                opts.will_save_wait_until.unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Generate a summary of supported features.
    pub fn feature_summary(&self) -> FeatureSummary {
        FeatureSummary {
            hover: self.supports_hover(),
            completion: self.supports_completion(),
            signature_help: self.supports_signature_help(),
            definition: self.supports_definition(),
            type_definition: self.supports_type_definition(),
            implementation: self.supports_implementation(),
            references: self.supports_references(),
            document_highlight: self.supports_document_highlight(),
            document_symbol: self.supports_document_symbol(),
            workspace_symbol: self.supports_workspace_symbol(),
            code_action: self.supports_code_action(),
            code_lens: self.supports_code_lens(),
            formatting: self.supports_formatting(),
            range_formatting: self.supports_range_formatting(),
            on_type_formatting: self.supports_on_type_formatting(),
            rename: self.supports_rename(),
            folding_range: self.supports_folding_range(),
            selection_range: self.supports_selection_range(),
            semantic_tokens: self.supports_semantic_tokens(),
            inlay_hints: self.supports_inlay_hints(),
            call_hierarchy: self.supports_call_hierarchy(),
        }
    }
}

/// Summary of features supported by a language server.
#[derive(Debug, Clone, Default)]
pub struct FeatureSummary {
    pub hover: bool,
    pub completion: bool,
    pub signature_help: bool,
    pub definition: bool,
    pub type_definition: bool,
    pub implementation: bool,
    pub references: bool,
    pub document_highlight: bool,
    pub document_symbol: bool,
    pub workspace_symbol: bool,
    pub code_action: bool,
    pub code_lens: bool,
    pub formatting: bool,
    pub range_formatting: bool,
    pub on_type_formatting: bool,
    pub rename: bool,
    pub folding_range: bool,
    pub selection_range: bool,
    pub semantic_tokens: bool,
    pub inlay_hints: bool,
    pub call_hierarchy: bool,
}

impl FeatureSummary {
    /// Count the number of supported features.
    pub fn supported_count(&self) -> usize {
        [
            self.hover,
            self.completion,
            self.signature_help,
            self.definition,
            self.type_definition,
            self.implementation,
            self.references,
            self.document_highlight,
            self.document_symbol,
            self.workspace_symbol,
            self.code_action,
            self.code_lens,
            self.formatting,
            self.range_formatting,
            self.on_type_formatting,
            self.rename,
            self.folding_range,
            self.selection_range,
            self.semantic_tokens,
            self.inlay_hints,
            self.call_hierarchy,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }

    /// Get the total number of features.
    pub fn total_count(&self) -> usize {
        21
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_capabilities() {
        let caps = ServerCapabilityInfo::default();
        assert!(!caps.supports_hover());
        assert!(!caps.supports_completion());
        assert!(!caps.supports_definition());
    }

    #[test]
    fn test_hover_capability() {
        let mut raw = ServerCapabilities::default();
        raw.hover_provider = Some(HoverProviderCapability::Simple(true));
        let caps = ServerCapabilityInfo::new(raw);
        assert!(caps.supports_hover());
    }

    #[test]
    fn test_text_document_sync() {
        let mut raw = ServerCapabilities::default();
        raw.text_document_sync = Some(TextDocumentSyncCapability::Kind(
            TextDocumentSyncKind::INCREMENTAL,
        ));
        let caps = ServerCapabilityInfo::new(raw);
        assert_eq!(caps.text_document_sync_kind(), TextDocumentSyncKind::INCREMENTAL);
    }

    #[test]
    fn test_feature_summary() {
        let caps = ServerCapabilityInfo::default();
        let summary = caps.feature_summary();
        assert_eq!(summary.supported_count(), 0);
        assert_eq!(summary.total_count(), 21);
    }
}
