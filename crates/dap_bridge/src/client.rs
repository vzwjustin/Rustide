//! DAP Client implementation

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Errors that can occur in the DAP client
#[derive(Error, Debug)]
pub enum DapError {
    #[error("Failed to spawn debug adapter: {0}")]
    SpawnError(String),

    #[error("Failed to send request: {0}")]
    SendError(String),

    #[error("Failed to receive response: {0}")]
    ReceiveError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Debug adapter not initialized")]
    NotInitialized,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Debug session error: {0}")]
    SessionError(String),
}

/// Configuration for the DAP client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapClientConfig {
    /// Path to the debug adapter executable
    pub adapter_path: PathBuf,
    /// Arguments to pass to the adapter
    pub adapter_args: Vec<String>,
    /// Working directory for the adapter
    pub working_dir: Option<PathBuf>,
    /// Environment variables for the adapter
    pub env: HashMap<String, String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for DapClientConfig {
    fn default() -> Self {
        Self {
            adapter_path: PathBuf::from("lldb-vscode"),
            adapter_args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            timeout_ms: 10000,
        }
    }
}

/// A DAP protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DapMessage {
    #[serde(rename = "request")]
    Request {
        seq: i64,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        arguments: Option<Value>,
    },
    #[serde(rename = "response")]
    Response {
        seq: i64,
        request_seq: i64,
        success: bool,
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
    #[serde(rename = "event")]
    Event {
        seq: i64,
        event: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
    },
}

/// Pending request waiting for response
struct PendingRequest {
    response_tx: oneshot::Sender<Result<Value, DapError>>,
}

/// The DAP client
pub struct DapClient {
    config: DapClientConfig,
    process: Option<Child>,
    request_tx: Option<mpsc::Sender<DapMessage>>,
    pending_requests: Arc<RwLock<HashMap<i64, PendingRequest>>>,
    event_tx: Option<mpsc::Sender<DapMessage>>,
    seq: AtomicI64,
    initialized: Arc<RwLock<bool>>,
}

impl DapClient {
    /// Create a new DAP client with the given configuration
    pub fn new(config: DapClientConfig) -> Self {
        Self {
            config,
            process: None,
            request_tx: None,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            event_tx: None,
            seq: AtomicI64::new(1),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the debug adapter process
    pub async fn start(&mut self) -> Result<mpsc::Receiver<DapMessage>, DapError> {
        info!("Starting debug adapter: {:?}", self.config.adapter_path);

        let mut cmd = Command::new(&self.config.adapter_path);
        cmd.args(&self.config.adapter_args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        if let Some(ref dir) = self.config.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &self.config.env {
            cmd.env(key, value);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| DapError::SpawnError(e.to_string()))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            DapError::SpawnError("Failed to capture stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            DapError::SpawnError("Failed to capture stdout".to_string())
        })?;

        // Channel for sending requests
        let (request_tx, request_rx) = mpsc::channel::<DapMessage>(32);

        // Channel for receiving events
        let (event_tx, event_rx) = mpsc::channel::<DapMessage>(32);

        let pending_requests = self.pending_requests.clone();

        // Spawn writer task
        tokio::spawn(Self::writer_task(stdin, request_rx));

        // Spawn reader task
        tokio::spawn(Self::reader_task(
            stdout,
            event_tx.clone(),
            pending_requests,
        ));

        self.process = Some(child);
        self.request_tx = Some(request_tx);
        self.event_tx = Some(event_tx);

        Ok(event_rx)
    }

    /// Writer task that sends messages to the adapter
    async fn writer_task(
        mut stdin: ChildStdin,
        mut request_rx: mpsc::Receiver<DapMessage>,
    ) {
        while let Some(msg) = request_rx.recv().await {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                    continue;
                }
            };

            let header = format!("Content-Length: {}\r\n\r\n", json.len());

            if let Err(e) = stdin.write_all(header.as_bytes()).await {
                error!("Failed to write header: {}", e);
                break;
            }

            if let Err(e) = stdin.write_all(json.as_bytes()).await {
                error!("Failed to write body: {}", e);
                break;
            }

            if let Err(e) = stdin.flush().await {
                error!("Failed to flush: {}", e);
                break;
            }

            debug!("Sent: {}", json);
        }
    }

    /// Reader task that receives messages from the adapter
    async fn reader_task(
        stdout: ChildStdout,
        event_tx: mpsc::Sender<DapMessage>,
        pending_requests: Arc<RwLock<HashMap<i64, PendingRequest>>>,
    ) {
        let mut reader = BufReader::new(stdout);
        let mut header_buf = String::new();

        loop {
            header_buf.clear();

            // Read headers
            let mut content_length: Option<usize> = None;
            loop {
                header_buf.clear();
                match reader.read_line(&mut header_buf).await {
                    Ok(0) => return, // EOF
                    Ok(_) => {
                        let line = header_buf.trim();
                        if line.is_empty() {
                            break;
                        }
                        if let Some(len_str) = line.strip_prefix("Content-Length: ") {
                            content_length = len_str.parse().ok();
                        }
                    }
                    Err(e) => {
                        error!("Failed to read header: {}", e);
                        return;
                    }
                }
            }

            let content_length = match content_length {
                Some(len) => len,
                None => {
                    warn!("No Content-Length header");
                    continue;
                }
            };

            // Read body
            let mut body = vec![0u8; content_length];
            if let Err(e) = reader.read_exact(&mut body).await {
                error!("Failed to read body: {}", e);
                return;
            }

            let body_str = match String::from_utf8(body) {
                Ok(s) => s,
                Err(e) => {
                    error!("Invalid UTF-8 in body: {}", e);
                    continue;
                }
            };

            debug!("Received: {}", body_str);

            let msg: DapMessage = match serde_json::from_str(&body_str) {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    continue;
                }
            };

            match &msg {
                DapMessage::Response { request_seq, success, body, message, .. } => {
                    let pending = pending_requests.write().remove(request_seq);
                    if let Some(pending) = pending {
                        let result = if *success {
                            Ok(body.clone().unwrap_or(Value::Null))
                        } else {
                            Err(DapError::SessionError(
                                message.clone().unwrap_or_else(|| "Unknown error".to_string())
                            ))
                        };
                        let _ = pending.response_tx.send(result);
                    }
                }
                DapMessage::Event { .. } => {
                    if let Err(e) = event_tx.send(msg).await {
                        error!("Failed to forward event: {}", e);
                    }
                }
                _ => {}
            }
        }
    }

    /// Get the next sequence number
    fn next_seq(&self) -> i64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a request and wait for response
    pub async fn request(&self, command: &str, arguments: Option<Value>) -> Result<Value, DapError> {
        let request_tx = self.request_tx.as_ref()
            .ok_or(DapError::NotInitialized)?;

        let seq = self.next_seq();
        let msg = DapMessage::Request {
            seq,
            command: command.to_string(),
            arguments,
        };

        let (response_tx, response_rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.write();
            pending.insert(seq, PendingRequest { response_tx });
        }

        request_tx.send(msg).await
            .map_err(|e| DapError::SendError(e.to_string()))?;

        let timeout = tokio::time::Duration::from_millis(self.config.timeout_ms);
        tokio::time::timeout(timeout, response_rx)
            .await
            .map_err(|_| DapError::Timeout)?
            .map_err(|_| DapError::ReceiveError("Channel closed".to_string()))?
    }

    /// Initialize the debug adapter
    pub async fn initialize(&self) -> Result<Value, DapError> {
        let args = json!({
            "clientID": "rust-ide",
            "clientName": "Rust IDE",
            "adapterID": "rust",
            "pathFormat": "path",
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsVariablePaging": true,
            "supportsRunInTerminalRequest": true,
            "locale": "en-US"
        });

        let result = self.request("initialize", Some(args)).await?;
        *self.initialized.write() = true;
        Ok(result)
    }

    /// Launch a program
    pub async fn launch(&self, program: &str, args: &[String], cwd: Option<&str>) -> Result<Value, DapError> {
        let mut launch_args = json!({
            "program": program,
            "args": args,
            "stopOnEntry": false,
        });

        if let Some(cwd) = cwd {
            launch_args["cwd"] = json!(cwd);
        }

        self.request("launch", Some(launch_args)).await
    }

    /// Attach to a running process
    pub async fn attach(&self, pid: u32) -> Result<Value, DapError> {
        let args = json!({
            "pid": pid
        });
        self.request("attach", Some(args)).await
    }

    /// Continue execution
    pub async fn continue_execution(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("continue", Some(args)).await
    }

    /// Step over
    pub async fn next(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("next", Some(args)).await
    }

    /// Step into
    pub async fn step_in(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("stepIn", Some(args)).await
    }

    /// Step out
    pub async fn step_out(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("stepOut", Some(args)).await
    }

    /// Pause execution
    pub async fn pause(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("pause", Some(args)).await
    }

    /// Get threads
    pub async fn threads(&self) -> Result<Value, DapError> {
        self.request("threads", None).await
    }

    /// Get stack trace
    pub async fn stack_trace(&self, thread_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "threadId": thread_id
        });
        self.request("stackTrace", Some(args)).await
    }

    /// Get scopes
    pub async fn scopes(&self, frame_id: i64) -> Result<Value, DapError> {
        let args = json!({
            "frameId": frame_id
        });
        self.request("scopes", Some(args)).await
    }

    /// Get variables
    pub async fn variables(&self, variables_reference: i64) -> Result<Value, DapError> {
        let args = json!({
            "variablesReference": variables_reference
        });
        self.request("variables", Some(args)).await
    }

    /// Evaluate expression
    pub async fn evaluate(&self, expression: &str, frame_id: Option<i64>, context: Option<&str>) -> Result<Value, DapError> {
        let mut args = json!({
            "expression": expression
        });

        if let Some(frame_id) = frame_id {
            args["frameId"] = json!(frame_id);
        }
        if let Some(context) = context {
            args["context"] = json!(context);
        }

        self.request("evaluate", Some(args)).await
    }

    /// Disconnect from the debug adapter
    pub async fn disconnect(&self) -> Result<Value, DapError> {
        let args = json!({
            "terminateDebuggee": false
        });
        self.request("disconnect", Some(args)).await
    }

    /// Terminate the debuggee
    pub async fn terminate(&self) -> Result<Value, DapError> {
        self.request("terminate", None).await
    }

    /// Check if the client is initialized
    pub fn is_initialized(&self) -> bool {
        *self.initialized.read()
    }

    /// Stop the debug adapter
    pub async fn stop(&mut self) -> Result<(), DapError> {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill().await;
        }
        self.request_tx = None;
        self.event_tx = None;
        *self.initialized.write() = false;
        Ok(())
    }
}

impl Drop for DapClient {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            // Best effort cleanup
            let _ = process.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = DapClientConfig::default();
        assert_eq!(config.timeout_ms, 10000);
    }

    #[test]
    fn test_dap_message_serialization() {
        let msg = DapMessage::Request {
            seq: 1,
            command: "initialize".to_string(),
            arguments: Some(json!({"clientID": "test"})),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("initialize"));
    }
}
