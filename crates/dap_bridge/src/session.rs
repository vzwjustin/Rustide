//! Debug session management

use crate::breakpoints::BreakpointManager;
use crate::client::{DapClient, DapClientConfig, DapError, DapMessage};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// The reason why execution stopped
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoppedReason {
    Step,
    Breakpoint,
    Exception,
    Pause,
    Entry,
    Goto,
    FunctionBreakpoint,
    DataBreakpoint,
    InstructionBreakpoint,
}

impl Default for StoppedReason {
    fn default() -> Self {
        Self::Pause
    }
}

/// State of the debug session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// Session not started
    Inactive,
    /// Initializing the debug adapter
    Initializing,
    /// Ready to launch/attach
    Ready,
    /// Program is running
    Running,
    /// Execution is stopped (breakpoint, step, etc.)
    Stopped(StoppedReason),
    /// Session is terminated
    Terminated,
}

/// Thread information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub id: i64,
    pub name: String,
}

/// Stack frame information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: i64,
    pub name: String,
    pub source: Option<String>,
    pub line: Option<i64>,
    pub column: Option<i64>,
}

/// Variable information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: Option<String>,
    pub variables_reference: i64,
}

/// Scope information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub variables_reference: i64,
    pub expensive: bool,
}

/// Debug session callbacks
pub trait SessionCallback: Send + Sync {
    fn on_state_changed(&self, state: &SessionState);
    fn on_output(&self, category: &str, output: &str);
    fn on_breakpoint_hit(&self, breakpoint_id: i64, thread_id: i64);
}

/// A debug session
pub struct DebugSession {
    client: DapClient,
    state: Arc<RwLock<SessionState>>,
    threads: Arc<RwLock<HashMap<i64, ThreadInfo>>>,
    current_thread_id: Arc<RwLock<Option<i64>>>,
    breakpoint_manager: Arc<BreakpointManager>,
    capabilities: Arc<RwLock<Value>>,
    callback: Option<Arc<dyn SessionCallback>>,
}

impl DebugSession {
    /// Create a new debug session
    pub fn new(config: DapClientConfig) -> Self {
        Self {
            client: DapClient::new(config),
            state: Arc::new(RwLock::new(SessionState::Inactive)),
            threads: Arc::new(RwLock::new(HashMap::new())),
            current_thread_id: Arc::new(RwLock::new(None)),
            breakpoint_manager: Arc::new(BreakpointManager::new()),
            capabilities: Arc::new(RwLock::new(Value::Null)),
            callback: None,
        }
    }

    /// Set the session callback
    pub fn set_callback(&mut self, callback: Arc<dyn SessionCallback>) {
        self.callback = Some(callback);
    }

    /// Get the current state
    pub fn state(&self) -> SessionState {
        self.state.read().clone()
    }

    /// Set the state and notify callback
    fn set_state(&self, state: SessionState) {
        *self.state.write() = state.clone();
        if let Some(ref callback) = self.callback {
            callback.on_state_changed(&state);
        }
    }

    /// Get the breakpoint manager
    pub fn breakpoints(&self) -> Arc<BreakpointManager> {
        self.breakpoint_manager.clone()
    }

    /// Start the debug session
    pub async fn start(&mut self) -> Result<(), DapError> {
        info!("Starting debug session");
        self.set_state(SessionState::Initializing);

        let event_rx = self.client.start().await?;

        // Spawn event handler
        let state = self.state.clone();
        let threads = self.threads.clone();
        let current_thread = self.current_thread_id.clone();
        let callback = self.callback.clone();

        tokio::spawn(async move {
            Self::handle_events(event_rx, state, threads, current_thread, callback).await;
        });

        // Initialize
        let caps = self.client.initialize().await?;
        *self.capabilities.write() = caps;

        self.set_state(SessionState::Ready);
        info!("Debug session ready");
        Ok(())
    }

    /// Handle events from the debug adapter
    async fn handle_events(
        mut event_rx: mpsc::Receiver<DapMessage>,
        state: Arc<RwLock<SessionState>>,
        threads: Arc<RwLock<HashMap<i64, ThreadInfo>>>,
        current_thread: Arc<RwLock<Option<i64>>>,
        callback: Option<Arc<dyn SessionCallback>>,
    ) {
        while let Some(msg) = event_rx.recv().await {
            if let DapMessage::Event { event, body, .. } = msg {
                debug!("Received event: {}", event);

                match event.as_str() {
                    "initialized" => {
                        info!("Debug adapter initialized");
                    }
                    "stopped" => {
                        let reason = body
                            .as_ref()
                            .and_then(|b| b.get("reason"))
                            .and_then(|r| r.as_str())
                            .unwrap_or("unknown");

                        let thread_id = body
                            .as_ref()
                            .and_then(|b| b.get("threadId"))
                            .and_then(|t| t.as_i64());

                        let stopped_reason = match reason {
                            "step" => StoppedReason::Step,
                            "breakpoint" => StoppedReason::Breakpoint,
                            "exception" => StoppedReason::Exception,
                            "pause" => StoppedReason::Pause,
                            "entry" => StoppedReason::Entry,
                            "goto" => StoppedReason::Goto,
                            _ => StoppedReason::Pause,
                        };

                        *state.write() = SessionState::Stopped(stopped_reason.clone());
                        *current_thread.write() = thread_id;

                        info!("Execution stopped: {:?}", stopped_reason);

                        if let Some(ref cb) = callback {
                            cb.on_state_changed(&SessionState::Stopped(stopped_reason));
                        }
                    }
                    "continued" => {
                        *state.write() = SessionState::Running;
                        if let Some(ref cb) = callback {
                            cb.on_state_changed(&SessionState::Running);
                        }
                    }
                    "exited" => {
                        let exit_code = body
                            .as_ref()
                            .and_then(|b| b.get("exitCode"))
                            .and_then(|c| c.as_i64())
                            .unwrap_or(0);
                        info!("Process exited with code: {}", exit_code);
                    }
                    "terminated" => {
                        *state.write() = SessionState::Terminated;
                        if let Some(ref cb) = callback {
                            cb.on_state_changed(&SessionState::Terminated);
                        }
                        info!("Debug session terminated");
                    }
                    "thread" => {
                        if let Some(ref body) = body {
                            let thread_id = body.get("threadId").and_then(|t| t.as_i64());
                            let reason = body.get("reason").and_then(|r| r.as_str());

                            if let (Some(id), Some(reason)) = (thread_id, reason) {
                                match reason {
                                    "started" => {
                                        threads.write().insert(
                                            id,
                                            ThreadInfo {
                                                id,
                                                name: format!("Thread {}", id),
                                            },
                                        );
                                    }
                                    "exited" => {
                                        threads.write().remove(&id);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    "output" => {
                        if let Some(ref body) = body {
                            let category = body
                                .get("category")
                                .and_then(|c| c.as_str())
                                .unwrap_or("console");
                            let output = body
                                .get("output")
                                .and_then(|o| o.as_str())
                                .unwrap_or("");

                            debug!("[{}] {}", category, output);

                            if let Some(ref cb) = callback {
                                cb.on_output(category, output);
                            }
                        }
                    }
                    _ => {
                        debug!("Unhandled event: {}", event);
                    }
                }
            }
        }
    }

    /// Launch a program
    pub async fn launch(
        &self,
        program: &str,
        args: &[String],
        cwd: Option<&str>,
    ) -> Result<(), DapError> {
        info!("Launching program: {}", program);
        self.client.launch(program, args, cwd).await?;
        self.set_state(SessionState::Running);
        Ok(())
    }

    /// Attach to a running process
    pub async fn attach(&self, pid: u32) -> Result<(), DapError> {
        info!("Attaching to process: {}", pid);
        self.client.attach(pid).await?;
        self.set_state(SessionState::Running);
        Ok(())
    }

    /// Continue execution
    pub async fn continue_execution(&self) -> Result<(), DapError> {
        let thread_id = self.current_thread_id.read().unwrap_or(0);
        self.client.continue_execution(thread_id).await?;
        self.set_state(SessionState::Running);
        Ok(())
    }

    /// Step over
    pub async fn step_over(&self) -> Result<(), DapError> {
        let thread_id = self.current_thread_id.read().unwrap_or(0);
        self.client.next(thread_id).await?;
        Ok(())
    }

    /// Step into
    pub async fn step_into(&self) -> Result<(), DapError> {
        let thread_id = self.current_thread_id.read().unwrap_or(0);
        self.client.step_in(thread_id).await?;
        Ok(())
    }

    /// Step out
    pub async fn step_out(&self) -> Result<(), DapError> {
        let thread_id = self.current_thread_id.read().unwrap_or(0);
        self.client.step_out(thread_id).await?;
        Ok(())
    }

    /// Pause execution
    pub async fn pause(&self) -> Result<(), DapError> {
        let thread_id = self.current_thread_id.read().unwrap_or(0);
        self.client.pause(thread_id).await?;
        Ok(())
    }

    /// Get all threads
    pub async fn get_threads(&self) -> Result<Vec<ThreadInfo>, DapError> {
        let response = self.client.threads().await?;

        let threads: Vec<ThreadInfo> = response
            .get("threads")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let id = t.get("id")?.as_i64()?;
                        let name = t.get("name")?.as_str()?.to_string();
                        Some(ThreadInfo { id, name })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Update cache
        let mut thread_map = self.threads.write();
        thread_map.clear();
        for thread in &threads {
            thread_map.insert(thread.id, thread.clone());
        }

        Ok(threads)
    }

    /// Get stack trace for a thread
    pub async fn get_stack_trace(&self, thread_id: i64) -> Result<Vec<StackFrame>, DapError> {
        let response = self.client.stack_trace(thread_id).await?;

        let frames = response
            .get("stackFrames")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| {
                        let id = f.get("id")?.as_i64()?;
                        let name = f.get("name")?.as_str()?.to_string();
                        let source = f
                            .get("source")
                            .and_then(|s| s.get("path"))
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string());
                        let line = f.get("line").and_then(|l| l.as_i64());
                        let column = f.get("column").and_then(|c| c.as_i64());

                        Some(StackFrame {
                            id,
                            name,
                            source,
                            line,
                            column,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(frames)
    }

    /// Get scopes for a stack frame
    pub async fn get_scopes(&self, frame_id: i64) -> Result<Vec<Scope>, DapError> {
        let response = self.client.scopes(frame_id).await?;

        let scopes = response
            .get("scopes")
            .and_then(|s| s.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let name = s.get("name")?.as_str()?.to_string();
                        let variables_reference =
                            s.get("variablesReference")?.as_i64()?;
                        let expensive = s.get("expensive").and_then(|e| e.as_bool()).unwrap_or(false);

                        Some(Scope {
                            name,
                            variables_reference,
                            expensive,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(scopes)
    }

    /// Get variables for a scope
    pub async fn get_variables(&self, variables_reference: i64) -> Result<Vec<Variable>, DapError> {
        let response = self.client.variables(variables_reference).await?;

        let variables = response
            .get("variables")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        let name = v.get("name")?.as_str()?.to_string();
                        let value = v.get("value")?.as_str()?.to_string();
                        let var_type = v.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
                        let variables_reference = v
                            .get("variablesReference")
                            .and_then(|r| r.as_i64())
                            .unwrap_or(0);

                        Some(Variable {
                            name,
                            value,
                            var_type,
                            variables_reference,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(variables)
    }

    /// Evaluate an expression
    pub async fn evaluate(
        &self,
        expression: &str,
        frame_id: Option<i64>,
    ) -> Result<String, DapError> {
        let response = self.client.evaluate(expression, frame_id, Some("repl")).await?;

        let result = response
            .get("result")
            .and_then(|r| r.as_str())
            .unwrap_or("")
            .to_string();

        Ok(result)
    }

    /// Set breakpoints in a file
    pub async fn set_file_breakpoints(&self, file: &PathBuf) -> Result<(), DapError> {
        let breakpoints = self.breakpoint_manager.get_breakpoints_for_file(file);

        let bp_args: Vec<Value> = breakpoints
            .iter()
            .map(|bp| {
                let mut obj = serde_json::json!({
                    "line": bp.line
                });
                if let Some(ref condition) = bp.condition {
                    obj["condition"] = serde_json::json!(condition);
                }
                obj
            })
            .collect();

        let args = serde_json::json!({
            "source": {
                "path": file.to_string_lossy()
            },
            "breakpoints": bp_args
        });

        self.client.request("setBreakpoints", Some(args)).await?;
        Ok(())
    }

    /// Stop the debug session
    pub async fn stop(&mut self) -> Result<(), DapError> {
        info!("Stopping debug session");
        let _ = self.client.disconnect().await;
        self.client.stop().await?;
        self.set_state(SessionState::Inactive);
        Ok(())
    }

    /// Terminate the debuggee
    pub async fn terminate(&self) -> Result<(), DapError> {
        self.client.terminate().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state() {
        let state = SessionState::Stopped(StoppedReason::Breakpoint);
        assert!(matches!(state, SessionState::Stopped(StoppedReason::Breakpoint)));
    }

    #[test]
    fn test_stopped_reason_default() {
        let reason = StoppedReason::default();
        assert_eq!(reason, StoppedReason::Pause);
    }
}
