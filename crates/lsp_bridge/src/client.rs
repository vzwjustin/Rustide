//! LSP client for communicating with language servers.
//!
//! This module provides the core client implementation for spawning LSP server
//! processes and handling JSON-RPC message communication.

use crate::capabilities::ServerCapabilityInfo;
use crate::protocol::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, LspMessage, RequestId,
};

use anyhow::{Context, Result};
use futures::channel::oneshot;
use lsp_types::{
    ClientCapabilities, InitializeParams, InitializeResult, InitializedParams,
    TextDocumentClientCapabilities, WindowClientCapabilities, WorkspaceClientCapabilities,
    GeneralClientCapabilities, Uri,
};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace, warn};

/// Errors that can occur during LSP client operations.
#[derive(Debug, Error)]
pub enum LspClientError {
    #[error("Failed to spawn server process: {0}")]
    SpawnError(String),

    #[error("Server process exited unexpectedly")]
    ServerExited,

    #[error("Failed to send message: {0}")]
    SendError(String),

    #[error("Failed to receive response: {0}")]
    ReceiveError(String),

    #[error("Request timed out")]
    Timeout,

    #[error("Request was cancelled")]
    Cancelled,

    #[error("Server returned error: {0}")]
    ServerError(JsonRpcError),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Client not initialized")]
    NotInitialized,

    #[error("Client is shutting down")]
    ShuttingDown,
}

/// Configuration for spawning an LSP server.
#[derive(Debug, Clone)]
pub struct LspServerConfig {
    /// Command to run (e.g., "rust-analyzer").
    pub command: String,
    /// Arguments to pass to the command.
    pub args: Vec<String>,
    /// Environment variables to set.
    pub env: HashMap<String, String>,
    /// Working directory for the server.
    pub working_dir: Option<PathBuf>,
    /// Root URI for the workspace.
    pub root_uri: Option<Uri>,
    /// Workspace folders.
    pub workspace_folders: Vec<lsp_types::WorkspaceFolder>,
}

impl LspServerConfig {
    /// Create a new server configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            working_dir: None,
            root_uri: None,
            workspace_folders: Vec::new(),
        }
    }

    /// Add an argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|a| a.into()));
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the working directory.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set the root URI.
    pub fn root_uri(mut self, uri: Uri) -> Self {
        self.root_uri = Some(uri);
        self
    }

    /// Add a workspace folder.
    pub fn workspace_folder(mut self, folder: lsp_types::WorkspaceFolder) -> Self {
        self.workspace_folders.push(folder);
        self
    }
}

/// Pending request tracker.
struct PendingRequest {
    sender: oneshot::Sender<Result<Value, JsonRpcError>>,
}

/// State of the LSP client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Client is not connected.
    Disconnected,
    /// Client is starting up.
    Starting,
    /// Client is connected but not initialized.
    Connected,
    /// Client is initializing.
    Initializing,
    /// Client is ready.
    Ready,
    /// Client is shutting down.
    ShuttingDown,
    /// Client has shut down.
    Shutdown,
}

/// Callback for server notifications.
pub type NotificationCallback = Arc<dyn Fn(&str, Option<Value>) + Send + Sync>;

/// An LSP client that communicates with a language server.
pub struct LspClient {
    /// Server configuration.
    config: LspServerConfig,
    /// Current state.
    state: Arc<Mutex<ClientState>>,
    /// Server process.
    process: Arc<Mutex<Option<Child>>>,
    /// Next request ID.
    next_id: AtomicI64,
    /// Pending requests waiting for responses.
    pending_requests: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    /// Server capabilities.
    capabilities: Arc<Mutex<Option<ServerCapabilityInfo>>>,
    /// Writer for sending messages to the server.
    writer: Arc<Mutex<Option<Box<dyn Write + Send>>>>,
    /// Shutdown flag.
    is_shutdown: AtomicBool,
    /// Notification callback.
    notification_callback: Arc<Mutex<Option<NotificationCallback>>>,
    /// Channel for outgoing messages.
    outgoing_tx: Option<mpsc::UnboundedSender<LspMessage>>,
}

impl LspClient {
    /// Create a new LSP client with the given configuration.
    pub fn new(config: LspServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ClientState::Disconnected)),
            process: Arc::new(Mutex::new(None)),
            next_id: AtomicI64::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            capabilities: Arc::new(Mutex::new(None)),
            writer: Arc::new(Mutex::new(None)),
            is_shutdown: AtomicBool::new(false),
            notification_callback: Arc::new(Mutex::new(None)),
            outgoing_tx: None,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> ClientState {
        *self.state.lock()
    }

    /// Check if the client is ready.
    pub fn is_ready(&self) -> bool {
        self.state() == ClientState::Ready
    }

    /// Set the notification callback.
    pub fn set_notification_callback<F>(&self, callback: F)
    where
        F: Fn(&str, Option<Value>) + Send + Sync + 'static,
    {
        *self.notification_callback.lock() = Some(Arc::new(callback));
    }

    /// Get the server capabilities.
    pub fn capabilities(&self) -> Option<ServerCapabilityInfo> {
        self.capabilities.lock().clone()
    }

    /// Start the language server and initialize it.
    pub async fn start(&mut self) -> Result<InitializeResult> {
        info!("Starting LSP server: {}", self.config.command);
        *self.state.lock() = ClientState::Starting;

        // Spawn the server process
        let mut cmd = Command::new(&self.config.command);
        cmd.args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().map_err(|e| {
            *self.state.lock() = ClientState::Disconnected;
            LspClientError::SpawnError(e.to_string())
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            LspClientError::SpawnError("Failed to get stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            LspClientError::SpawnError("Failed to get stdout".to_string())
        })?;

        *self.writer.lock() = Some(Box::new(stdin));
        *self.process.lock() = Some(child);
        *self.state.lock() = ClientState::Connected;

        // Start the reader thread
        self.start_reader_thread(stdout);

        // Initialize the server
        *self.state.lock() = ClientState::Initializing;
        let result = self.initialize().await?;

        // Store capabilities
        *self.capabilities.lock() = Some(ServerCapabilityInfo::new(result.capabilities.clone()));

        // Send initialized notification
        self.notify::<lsp_types::notification::Initialized>(InitializedParams {})?;

        *self.state.lock() = ClientState::Ready;
        info!("LSP server initialized successfully");

        Ok(result)
    }

    /// Start the reader thread for processing incoming messages.
    fn start_reader_thread(&self, stdout: std::process::ChildStdout) {
        let pending_requests = Arc::clone(&self.pending_requests);
        let state = Arc::clone(&self.state);
        let notification_callback = Arc::clone(&self.notification_callback);

        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut headers = String::new();

            loop {
                headers.clear();

                // Read headers
                let mut content_length: Option<usize> = None;
                loop {
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(0) => {
                            debug!("Server closed stdout");
                            *state.lock() = ClientState::Shutdown;
                            return;
                        }
                        Ok(_) => {
                            let line = line.trim();
                            if line.is_empty() {
                                break;
                            }
                            if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                                content_length = len_str.parse().ok();
                            }
                        }
                        Err(e) => {
                            error!("Error reading from server: {}", e);
                            *state.lock() = ClientState::Shutdown;
                            return;
                        }
                    }
                }

                let content_length = match content_length {
                    Some(len) => len,
                    None => {
                        warn!("Missing Content-Length header");
                        continue;
                    }
                };

                // Read content
                let mut content = vec![0u8; content_length];
                if let Err(e) = std::io::Read::read_exact(&mut reader, &mut content) {
                    error!("Error reading content: {}", e);
                    *state.lock() = ClientState::Shutdown;
                    return;
                }

                let content = match String::from_utf8(content) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Invalid UTF-8 in response: {}", e);
                        continue;
                    }
                };

                trace!("Received: {}", content);

                // Parse the message
                let message: Value = match serde_json::from_str(&content) {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to parse JSON: {}", e);
                        continue;
                    }
                };

                // Handle the message
                if message.get("id").is_some() && message.get("method").is_some() {
                    // It's a request from the server
                    debug!("Received server request: {:?}", message.get("method"));
                    // TODO: Handle server requests
                } else if message.get("id").is_some() {
                    // It's a response
                    if let Some(id) = message.get("id") {
                        let request_id = if let Some(n) = id.as_i64() {
                            RequestId::Number(n)
                        } else if let Some(s) = id.as_str() {
                            RequestId::String(s.to_string())
                        } else {
                            continue;
                        };

                        let mut pending = pending_requests.lock();
                        if let Some(request) = pending.remove(&request_id) {
                            let result = if let Some(error) = message.get("error") {
                                match serde_json::from_value::<JsonRpcError>(error.clone()) {
                                    Ok(e) => Err(e),
                                    Err(_) => Err(JsonRpcError::internal_error("Invalid error")),
                                }
                            } else {
                                Ok(message.get("result").cloned().unwrap_or(Value::Null))
                            };
                            let _ = request.sender.send(result);
                        }
                    }
                } else if message.get("method").is_some() {
                    // It's a notification
                    let method = message.get("method").and_then(|m| m.as_str()).unwrap_or("");
                    let params = message.get("params").cloned();

                    debug!("Received notification: {}", method);

                    if let Some(callback) = notification_callback.lock().as_ref() {
                        callback(method, params);
                    }
                }
            }
        });
    }

    /// Generate the next request ID.
    fn next_request_id(&self) -> RequestId {
        RequestId::Number(self.next_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Send a raw message to the server.
    fn send_raw(&self, content: &str) -> Result<()> {
        let mut writer = self.writer.lock();
        let writer = writer.as_mut().ok_or(LspClientError::NotInitialized)?;

        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
        trace!("Sending: {}", content);

        writer
            .write_all(message.as_bytes())
            .context("Failed to write to server")?;
        writer.flush().context("Failed to flush")?;

        Ok(())
    }

    /// Send a request and wait for the response.
    pub async fn request<R>(&self, params: R::Params) -> Result<R::Result>
    where
        R: lsp_types::request::Request,
        R::Params: Serialize,
        R::Result: DeserializeOwned,
    {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(LspClientError::ShuttingDown.into());
        }

        let id = self.next_request_id();
        let params_value = serde_json::to_value(&params)
            .map_err(|e| LspClientError::SerializationError(e.to_string()))?;

        let request = JsonRpcRequest::new(id.clone(), R::METHOD, Some(params_value));
        let content = serde_json::to_string(&request)
            .map_err(|e| LspClientError::SerializationError(e.to_string()))?;

        // Create a oneshot channel for the response
        let (tx, rx) = oneshot::channel();
        self.pending_requests.lock().insert(id.clone(), PendingRequest { sender: tx });

        // Send the request
        self.send_raw(&content)?;

        // Wait for the response
        let result = rx.await.map_err(|_| LspClientError::ReceiveError("Channel closed".to_string()))?;

        match result {
            Ok(value) => {
                serde_json::from_value(value)
                    .map_err(|e| LspClientError::SerializationError(e.to_string()).into())
            }
            Err(error) => Err(LspClientError::ServerError(error).into()),
        }
    }

    /// Send a notification (no response expected).
    pub fn notify<N>(&self, params: N::Params) -> Result<()>
    where
        N: lsp_types::notification::Notification,
        N::Params: Serialize,
    {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(LspClientError::ShuttingDown.into());
        }

        let params_value = serde_json::to_value(&params)
            .map_err(|e| LspClientError::SerializationError(e.to_string()))?;

        let notification = JsonRpcNotification::new(N::METHOD, Some(params_value));
        let content = serde_json::to_string(&notification)
            .map_err(|e| LspClientError::SerializationError(e.to_string()))?;

        self.send_raw(&content)
    }

    /// Initialize the server.
    async fn initialize(&self) -> Result<InitializeResult> {
        let root_uri = self.config.root_uri.clone();

        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_path: None,
            root_uri,
            initialization_options: None,
            capabilities: Self::client_capabilities(),
            trace: Some(lsp_types::TraceValue::Off),
            workspace_folders: if self.config.workspace_folders.is_empty() {
                None
            } else {
                Some(self.config.workspace_folders.clone())
            },
            client_info: Some(lsp_types::ClientInfo {
                name: "RustIDE".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            locale: None,
            work_done_progress_params: Default::default(),
        };

        self.request::<lsp_types::request::Initialize>(params).await
    }

    /// Generate client capabilities.
    fn client_capabilities() -> ClientCapabilities {
        ClientCapabilities {
            workspace: Some(WorkspaceClientCapabilities {
                apply_edit: Some(true),
                workspace_edit: Some(lsp_types::WorkspaceEditClientCapabilities {
                    document_changes: Some(true),
                    ..Default::default()
                }),
                did_change_configuration: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(true),
                }),
                did_change_watched_files: Some(lsp_types::DidChangeWatchedFilesClientCapabilities {
                    dynamic_registration: Some(true),
                    relative_pattern_support: Some(true),
                }),
                symbol: Some(lsp_types::WorkspaceSymbolClientCapabilities {
                    dynamic_registration: Some(true),
                    ..Default::default()
                }),
                workspace_folders: Some(true),
                configuration: Some(true),
                ..Default::default()
            }),
            text_document: Some(TextDocumentClientCapabilities {
                synchronization: Some(lsp_types::TextDocumentSyncClientCapabilities {
                    dynamic_registration: Some(true),
                    will_save: Some(true),
                    will_save_wait_until: Some(true),
                    did_save: Some(true),
                }),
                completion: Some(lsp_types::CompletionClientCapabilities {
                    dynamic_registration: Some(true),
                    completion_item: Some(lsp_types::CompletionItemCapability {
                        snippet_support: Some(true),
                        commit_characters_support: Some(true),
                        documentation_format: Some(vec![
                            lsp_types::MarkupKind::Markdown,
                            lsp_types::MarkupKind::PlainText,
                        ]),
                        deprecated_support: Some(true),
                        preselect_support: Some(true),
                        ..Default::default()
                    }),
                    context_support: Some(true),
                    ..Default::default()
                }),
                hover: Some(lsp_types::HoverClientCapabilities {
                    dynamic_registration: Some(true),
                    content_format: Some(vec![
                        lsp_types::MarkupKind::Markdown,
                        lsp_types::MarkupKind::PlainText,
                    ]),
                }),
                signature_help: Some(lsp_types::SignatureHelpClientCapabilities {
                    dynamic_registration: Some(true),
                    signature_information: Some(lsp_types::SignatureInformationSettings {
                        documentation_format: Some(vec![
                            lsp_types::MarkupKind::Markdown,
                            lsp_types::MarkupKind::PlainText,
                        ]),
                        parameter_information: Some(lsp_types::ParameterInformationSettings {
                            label_offset_support: Some(true),
                        }),
                        active_parameter_support: Some(true),
                    }),
                    context_support: Some(true),
                }),
                declaration: Some(lsp_types::GotoCapability {
                    dynamic_registration: Some(true),
                    link_support: Some(true),
                }),
                definition: Some(lsp_types::GotoCapability {
                    dynamic_registration: Some(true),
                    link_support: Some(true),
                }),
                type_definition: Some(lsp_types::GotoCapability {
                    dynamic_registration: Some(true),
                    link_support: Some(true),
                }),
                implementation: Some(lsp_types::GotoCapability {
                    dynamic_registration: Some(true),
                    link_support: Some(true),
                }),
                references: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(true),
                }),
                document_highlight: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(true),
                }),
                document_symbol: Some(lsp_types::DocumentSymbolClientCapabilities {
                    dynamic_registration: Some(true),
                    hierarchical_document_symbol_support: Some(true),
                    ..Default::default()
                }),
                code_action: Some(lsp_types::CodeActionClientCapabilities {
                    dynamic_registration: Some(true),
                    code_action_literal_support: Some(lsp_types::CodeActionLiteralSupport {
                        code_action_kind: lsp_types::CodeActionKindLiteralSupport {
                            value_set: vec![
                                lsp_types::CodeActionKind::QUICKFIX.as_str().to_string(),
                                lsp_types::CodeActionKind::REFACTOR.as_str().to_string(),
                                lsp_types::CodeActionKind::REFACTOR_EXTRACT.as_str().to_string(),
                                lsp_types::CodeActionKind::REFACTOR_INLINE.as_str().to_string(),
                                lsp_types::CodeActionKind::REFACTOR_REWRITE.as_str().to_string(),
                                lsp_types::CodeActionKind::SOURCE.as_str().to_string(),
                                lsp_types::CodeActionKind::SOURCE_ORGANIZE_IMPORTS.as_str().to_string(),
                            ],
                        },
                    }),
                    is_preferred_support: Some(true),
                    ..Default::default()
                }),
                formatting: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(true),
                }),
                range_formatting: Some(lsp_types::DynamicRegistrationClientCapabilities {
                    dynamic_registration: Some(true),
                }),
                rename: Some(lsp_types::RenameClientCapabilities {
                    dynamic_registration: Some(true),
                    prepare_support: Some(true),
                    ..Default::default()
                }),
                publish_diagnostics: Some(lsp_types::PublishDiagnosticsClientCapabilities {
                    related_information: Some(true),
                    tag_support: Some(lsp_types::TagSupport {
                        value_set: vec![
                            lsp_types::DiagnosticTag::UNNECESSARY,
                            lsp_types::DiagnosticTag::DEPRECATED,
                        ],
                    }),
                    version_support: Some(true),
                    code_description_support: Some(true),
                    data_support: Some(true),
                }),
                folding_range: Some(lsp_types::FoldingRangeClientCapabilities {
                    dynamic_registration: Some(true),
                    ..Default::default()
                }),
                semantic_tokens: Some(lsp_types::SemanticTokensClientCapabilities {
                    dynamic_registration: Some(true),
                    requests: lsp_types::SemanticTokensClientCapabilitiesRequests {
                        range: Some(true),
                        full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    },
                    token_types: vec![
                        lsp_types::SemanticTokenType::NAMESPACE,
                        lsp_types::SemanticTokenType::TYPE,
                        lsp_types::SemanticTokenType::CLASS,
                        lsp_types::SemanticTokenType::ENUM,
                        lsp_types::SemanticTokenType::INTERFACE,
                        lsp_types::SemanticTokenType::STRUCT,
                        lsp_types::SemanticTokenType::TYPE_PARAMETER,
                        lsp_types::SemanticTokenType::PARAMETER,
                        lsp_types::SemanticTokenType::VARIABLE,
                        lsp_types::SemanticTokenType::PROPERTY,
                        lsp_types::SemanticTokenType::ENUM_MEMBER,
                        lsp_types::SemanticTokenType::EVENT,
                        lsp_types::SemanticTokenType::FUNCTION,
                        lsp_types::SemanticTokenType::METHOD,
                        lsp_types::SemanticTokenType::MACRO,
                        lsp_types::SemanticTokenType::KEYWORD,
                        lsp_types::SemanticTokenType::MODIFIER,
                        lsp_types::SemanticTokenType::COMMENT,
                        lsp_types::SemanticTokenType::STRING,
                        lsp_types::SemanticTokenType::NUMBER,
                        lsp_types::SemanticTokenType::REGEXP,
                        lsp_types::SemanticTokenType::OPERATOR,
                    ],
                    token_modifiers: vec![
                        lsp_types::SemanticTokenModifier::DECLARATION,
                        lsp_types::SemanticTokenModifier::DEFINITION,
                        lsp_types::SemanticTokenModifier::READONLY,
                        lsp_types::SemanticTokenModifier::STATIC,
                        lsp_types::SemanticTokenModifier::DEPRECATED,
                        lsp_types::SemanticTokenModifier::ABSTRACT,
                        lsp_types::SemanticTokenModifier::ASYNC,
                        lsp_types::SemanticTokenModifier::MODIFICATION,
                        lsp_types::SemanticTokenModifier::DOCUMENTATION,
                        lsp_types::SemanticTokenModifier::DEFAULT_LIBRARY,
                    ],
                    formats: vec![lsp_types::TokenFormat::RELATIVE],
                    overlapping_token_support: Some(false),
                    multiline_token_support: Some(true),
                    ..Default::default()
                }),
                inlay_hint: Some(lsp_types::InlayHintClientCapabilities {
                    dynamic_registration: Some(true),
                    resolve_support: None,
                }),
                ..Default::default()
            }),
            window: Some(WindowClientCapabilities {
                work_done_progress: Some(true),
                show_message: Some(lsp_types::ShowMessageRequestClientCapabilities {
                    message_action_item: Some(lsp_types::MessageActionItemCapabilities {
                        additional_properties_support: Some(true),
                    }),
                }),
                show_document: Some(lsp_types::ShowDocumentClientCapabilities {
                    support: true,
                }),
            }),
            general: Some(GeneralClientCapabilities {
                stale_request_support: Some(lsp_types::StaleRequestSupportClientCapabilities {
                    cancel: true,
                    retry_on_content_modified: vec![
                        "textDocument/semanticTokens/full".to_string(),
                        "textDocument/semanticTokens/range".to_string(),
                        "textDocument/semanticTokens/full/delta".to_string(),
                    ],
                }),
                regular_expressions: Some(lsp_types::RegularExpressionsClientCapabilities {
                    engine: "ECMAScript".to_string(),
                    version: Some("ES2020".to_string()),
                }),
                markdown: Some(lsp_types::MarkdownClientCapabilities {
                    parser: "marked".to_string(),
                    version: Some("1.1.0".to_string()),
                    allowed_tags: None,
                }),
                position_encodings: Some(vec![lsp_types::PositionEncodingKind::UTF16]),
            }),
            notebook_document: None,
            experimental: None,
        }
    }

    /// Shutdown the server.
    pub async fn shutdown(&self) -> Result<()> {
        if self.is_shutdown.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        info!("Shutting down LSP server");
        *self.state.lock() = ClientState::ShuttingDown;

        // Send shutdown request
        let _: () = self.request::<lsp_types::request::Shutdown>(()).await?;

        // Send exit notification
        self.notify::<lsp_types::notification::Exit>(())?;

        *self.state.lock() = ClientState::Shutdown;
        Ok(())
    }

    /// Force kill the server process.
    pub fn kill(&self) {
        if let Some(ref mut child) = *self.process.lock() {
            let _ = child.kill();
        }
        *self.state.lock() = ClientState::Shutdown;
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config() {
        let config = LspServerConfig::new("rust-analyzer")
            .arg("--version")
            .env("RUST_LOG", "debug")
            .working_dir("/tmp");

        assert_eq!(config.command, "rust-analyzer");
        assert_eq!(config.args, vec!["--version"]);
        assert_eq!(config.env.get("RUST_LOG"), Some(&"debug".to_string()));
        assert_eq!(config.working_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_client_creation() {
        let config = LspServerConfig::new("test-server");
        let client = LspClient::new(config);

        assert_eq!(client.state(), ClientState::Disconnected);
        assert!(!client.is_ready());
    }
}
