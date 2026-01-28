//! Server manager for managing multiple LSP server instances.
//!
//! This module provides lifecycle management for language servers,
//! including starting, stopping, and restarting servers on a per-language basis.

use crate::client::{ClientState, LspClient, LspServerConfig};
use crate::diagnostics::DiagnosticStore;

use anyhow::Result;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, InitializeResult, PublishDiagnosticsParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    VersionedTextDocumentIdentifier, Uri,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

/// Errors that can occur during server management.
#[derive(Debug, Error)]
pub enum ManagerError {
    #[error("No server configured for language: {0}")]
    NoServerConfigured(String),

    #[error("Server not running for language: {0}")]
    ServerNotRunning(String),

    #[error("Server already running for language: {0}")]
    ServerAlreadyRunning(String),

    #[error("Failed to start server: {0}")]
    StartError(String),

    #[error("Failed to stop server: {0}")]
    StopError(String),
}

/// Configuration for a language's LSP server.
#[derive(Debug, Clone)]
pub struct LanguageServerConfig {
    /// Language ID (e.g., "rust", "python").
    pub language_id: String,
    /// File extensions this server handles.
    pub file_extensions: Vec<String>,
    /// Server configuration.
    pub server_config: LspServerConfig,
}

impl LanguageServerConfig {
    /// Create a new language server configuration.
    pub fn new(language_id: impl Into<String>, config: LspServerConfig) -> Self {
        Self {
            language_id: language_id.into(),
            file_extensions: Vec::new(),
            server_config: config,
        }
    }

    /// Add a file extension.
    pub fn extension(mut self, ext: impl Into<String>) -> Self {
        self.file_extensions.push(ext.into());
        self
    }

    /// Add multiple file extensions.
    pub fn extensions(mut self, exts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.file_extensions.extend(exts.into_iter().map(|e| e.into()));
        self
    }
}

/// Information about a managed server.
pub struct ManagedServer {
    /// Language ID.
    pub language_id: String,
    /// The LSP client.
    pub client: LspClient,
    /// Initialize result.
    pub init_result: Option<InitializeResult>,
    /// Open documents.
    pub open_documents: HashMap<Uri, i32>,
}

impl ManagedServer {
    /// Create a new managed server.
    fn new(language_id: String, client: LspClient) -> Self {
        Self {
            language_id,
            client,
            init_result: None,
            open_documents: HashMap::new(),
        }
    }
}

/// Manages multiple LSP server instances.
pub struct ServerManager {
    /// Workspace root path.
    workspace_root: PathBuf,
    /// Workspace folders.
    workspace_folders: Vec<lsp_types::WorkspaceFolder>,
    /// Language server configurations.
    configs: RwLock<HashMap<String, LanguageServerConfig>>,
    /// Running servers.
    servers: RwLock<HashMap<String, ManagedServer>>,
    /// Extension to language ID mapping.
    extension_map: RwLock<HashMap<String, String>>,
    /// Diagnostic store.
    diagnostics: Arc<DiagnosticStore>,
}

impl ServerManager {
    /// Create a new server manager.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        Self {
            workspace_root,
            workspace_folders: Vec::new(),
            configs: RwLock::new(HashMap::new()),
            servers: RwLock::new(HashMap::new()),
            extension_map: RwLock::new(HashMap::new()),
            diagnostics: Arc::new(DiagnosticStore::new()),
        }
    }

    /// Get the diagnostic store.
    pub fn diagnostics(&self) -> Arc<DiagnosticStore> {
        Arc::clone(&self.diagnostics)
    }

    /// Add a workspace folder.
    pub fn add_workspace_folder(&mut self, folder: lsp_types::WorkspaceFolder) {
        self.workspace_folders.push(folder);
    }

    /// Register a language server configuration.
    pub fn register_server(&self, config: LanguageServerConfig) {
        let language_id = config.language_id.clone();

        // Update extension map
        {
            let mut ext_map = self.extension_map.write();
            for ext in &config.file_extensions {
                ext_map.insert(ext.clone(), language_id.clone());
            }
        }

        // Store config
        self.configs.write().insert(language_id, config);
    }

    /// Get the language ID for a file.
    pub fn language_id_for_file(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| self.extension_map.read().get(ext).cloned())
    }

    /// Start a language server.
    pub async fn start_server(&self, language_id: &str) -> Result<()> {
        // Check if already running
        if self.servers.read().contains_key(language_id) {
            return Err(ManagerError::ServerAlreadyRunning(language_id.to_string()).into());
        }

        // Get config
        let config = self
            .configs
            .read()
            .get(language_id)
            .cloned()
            .ok_or_else(|| ManagerError::NoServerConfigured(language_id.to_string()))?;

        info!("Starting language server for: {}", language_id);

        // Create client with workspace info
        let mut server_config = config.server_config.clone();
        if let Some(uri) = path_to_uri(&self.workspace_root) {
            server_config = server_config.root_uri(uri);
        }
        for folder in &self.workspace_folders {
            server_config = server_config.workspace_folder(folder.clone());
        }

        let mut client = LspClient::new(server_config);

        // Set up notification handler for diagnostics
        let diagnostics = Arc::clone(&self.diagnostics);
        client.set_notification_callback(move |method, params| {
            if method == "textDocument/publishDiagnostics" {
                if let Some(params) = params {
                    if let Ok(diag_params) = serde_json::from_value::<PublishDiagnosticsParams>(params) {
                        diagnostics.set_diagnostics(diag_params.uri, diag_params.diagnostics);
                    }
                }
            }
        });

        // Start the client
        let init_result = client
            .start()
            .await
            .map_err(|e| ManagerError::StartError(e.to_string()))?;

        // Store the managed server
        let mut server = ManagedServer::new(language_id.to_string(), client);
        server.init_result = Some(init_result);

        self.servers.write().insert(language_id.to_string(), server);

        info!("Language server started for: {}", language_id);
        Ok(())
    }

    /// Stop a language server.
    pub async fn stop_server(&self, language_id: &str) -> Result<()> {
        let mut servers = self.servers.write();
        let server = servers
            .remove(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        info!("Stopping language server for: {}", language_id);

        server
            .client
            .shutdown()
            .await
            .map_err(|e| ManagerError::StopError(e.to_string()))?;

        info!("Language server stopped for: {}", language_id);
        Ok(())
    }

    /// Restart a language server.
    pub async fn restart_server(&self, language_id: &str) -> Result<()> {
        // Stop if running
        if self.servers.read().contains_key(language_id) {
            self.stop_server(language_id).await?;
        }

        // Start again
        self.start_server(language_id).await
    }

    /// Check if a server is running.
    pub fn is_server_running(&self, language_id: &str) -> bool {
        self.servers
            .read()
            .get(language_id)
            .is_some_and(|s| s.client.is_ready())
    }

    /// Get the state of a server.
    pub fn server_state(&self, language_id: &str) -> Option<ClientState> {
        self.servers.read().get(language_id).map(|s| s.client.state())
    }

    /// Get a list of running servers.
    pub fn running_servers(&self) -> Vec<String> {
        self.servers
            .read()
            .iter()
            .filter(|(_, s)| s.client.is_ready())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Notify that a document was opened.
    pub fn did_open(&self, language_id: &str, uri: Uri, text: String, version: i32) -> Result<()> {
        let mut servers = self.servers.write();
        let server = servers
            .get_mut(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: language_id.to_string(),
                version,
                text,
            },
        };

        server
            .client
            .notify::<lsp_types::notification::DidOpenTextDocument>(params)?;
        server.open_documents.insert(uri, version);

        Ok(())
    }

    /// Notify that a document changed.
    pub fn did_change(
        &self,
        language_id: &str,
        uri: Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<()> {
        let mut servers = self.servers.write();
        let server = servers
            .get_mut(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri: uri.clone(), version },
            content_changes: changes,
        };

        server
            .client
            .notify::<lsp_types::notification::DidChangeTextDocument>(params)?;
        server.open_documents.insert(uri, version);

        Ok(())
    }

    /// Notify that a document was saved.
    pub fn did_save(&self, language_id: &str, uri: Uri, text: Option<String>) -> Result<()> {
        let servers = self.servers.read();
        let server = servers
            .get(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text,
        };

        server
            .client
            .notify::<lsp_types::notification::DidSaveTextDocument>(params)?;

        Ok(())
    }

    /// Notify that a document was closed.
    pub fn did_close(&self, language_id: &str, uri: Uri) -> Result<()> {
        let mut servers = self.servers.write();
        let server = servers
            .get_mut(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };

        server
            .client
            .notify::<lsp_types::notification::DidCloseTextDocument>(params)?;
        server.open_documents.remove(&uri);

        // Clear diagnostics for closed file
        self.diagnostics.clear_diagnostics(&uri);

        Ok(())
    }

    /// Execute a request on a language server.
    pub async fn request<R>(&self, language_id: &str, params: R::Params) -> Result<R::Result>
    where
        R: lsp_types::request::Request,
        R::Params: serde::Serialize + Send + 'static,
        R::Result: serde::de::DeserializeOwned + Send + 'static,
    {
        let servers = self.servers.read();
        let server = servers
            .get(language_id)
            .ok_or_else(|| ManagerError::ServerNotRunning(language_id.to_string()))?;

        server.client.request::<R>(params).await
    }

    /// Stop all servers.
    pub async fn stop_all(&self) -> Result<()> {
        let language_ids: Vec<String> = self.servers.read().keys().cloned().collect();

        for language_id in language_ids {
            if let Err(e) = self.stop_server(&language_id).await {
                warn!("Failed to stop server for {}: {}", language_id, e);
            }
        }

        Ok(())
    }
}

/// Builder for creating pre-configured server managers.
pub struct ServerManagerBuilder {
    workspace_root: PathBuf,
    workspace_folders: Vec<lsp_types::WorkspaceFolder>,
    configs: Vec<LanguageServerConfig>,
}

impl ServerManagerBuilder {
    /// Create a new builder.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            workspace_folders: Vec::new(),
            configs: Vec::new(),
        }
    }

    /// Add a workspace folder.
    pub fn workspace_folder(mut self, folder: lsp_types::WorkspaceFolder) -> Self {
        self.workspace_folders.push(folder);
        self
    }

    /// Add a language server configuration.
    pub fn server(mut self, config: LanguageServerConfig) -> Self {
        self.configs.push(config);
        self
    }

    /// Add rust-analyzer configuration.
    pub fn with_rust_analyzer(self) -> Self {
        let config = LanguageServerConfig::new(
            "rust",
            LspServerConfig::new("rust-analyzer"),
        )
        .extension("rs");

        self.server(config)
    }

    /// Add Python LSP configuration.
    pub fn with_python_lsp(self) -> Self {
        let config = LanguageServerConfig::new(
            "python",
            LspServerConfig::new("pylsp"),
        )
        .extensions(["py", "pyi"]);

        self.server(config)
    }

    /// Add TypeScript LSP configuration.
    pub fn with_typescript_lsp(self) -> Self {
        let config = LanguageServerConfig::new(
            "typescript",
            LspServerConfig::new("typescript-language-server").arg("--stdio"),
        )
        .extensions(["ts", "tsx", "js", "jsx"]);

        self.server(config)
    }

    /// Add Go LSP configuration.
    pub fn with_gopls(self) -> Self {
        let config = LanguageServerConfig::new(
            "go",
            LspServerConfig::new("gopls"),
        )
        .extension("go");

        self.server(config)
    }

    /// Build the server manager.
    pub fn build(self) -> ServerManager {
        let mut manager = ServerManager::new(self.workspace_root);

        for folder in self.workspace_folders {
            manager.add_workspace_folder(folder);
        }

        for config in self.configs {
            manager.register_server(config);
        }

        manager
    }
}

/// Convert a path to a URI.
fn path_to_uri(path: &Path) -> Option<Uri> {
    let path_str = path.to_str()?;

    // Create a file:// URI from the path
    #[cfg(windows)]
    let uri_string = format!("file:///{}", path_str.replace('\\', "/"));
    #[cfg(not(windows))]
    let uri_string = format!("file://{}", path_str);

    Uri::from_str(&uri_string).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_server_config() {
        let config = LanguageServerConfig::new(
            "rust",
            LspServerConfig::new("rust-analyzer"),
        )
        .extension("rs")
        .extension("rlib");

        assert_eq!(config.language_id, "rust");
        assert_eq!(config.file_extensions, vec!["rs", "rlib"]);
    }

    #[test]
    fn test_server_manager_creation() {
        let manager = ServerManager::new("/tmp/workspace");
        assert!(manager.running_servers().is_empty());
    }

    #[test]
    fn test_extension_mapping() {
        let manager = ServerManager::new("/tmp/workspace");

        let config = LanguageServerConfig::new(
            "rust",
            LspServerConfig::new("rust-analyzer"),
        )
        .extension("rs");

        manager.register_server(config);

        assert_eq!(
            manager.language_id_for_file(Path::new("test.rs")),
            Some("rust".to_string())
        );
        assert_eq!(manager.language_id_for_file(Path::new("test.py")), None);
    }

    #[test]
    fn test_builder() {
        let manager = ServerManagerBuilder::new("/tmp/workspace")
            .with_rust_analyzer()
            .with_python_lsp()
            .build();

        assert_eq!(
            manager.language_id_for_file(Path::new("main.rs")),
            Some("rust".to_string())
        );
        assert_eq!(
            manager.language_id_for_file(Path::new("script.py")),
            Some("python".to_string())
        );
    }
}
