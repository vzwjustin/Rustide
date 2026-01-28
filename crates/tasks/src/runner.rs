//! Task runner implementation

use crate::output::{OutputLine, OutputParser, OutputType};
use crate::problem_matcher::{Problem, ProblemMatcher, RustProblemMatcher};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::{debug, error, info, warn};

/// Errors that can occur when running tasks
#[derive(Error, Debug)]
pub enum TaskError {
    #[error("Failed to spawn task: {0}")]
    SpawnError(String),

    #[error("Task not found: {0}")]
    NotFound(String),

    #[error("Task already running: {0}")]
    AlreadyRunning(String),

    #[error("Task failed with exit code: {0}")]
    Failed(i32),

    #[error("Task was cancelled")]
    Cancelled,

    #[error("IO error: {0}")]
    IoError(String),
}

/// Task status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is pending
    Pending,
    /// Task is running
    Running,
    /// Task completed successfully
    Success,
    /// Task failed with exit code
    Failed(i32),
    /// Task was cancelled
    Cancelled,
}

/// Task type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Build,
    Test,
    Run,
    Check,
    Clean,
    Custom,
}

/// Task configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name
    pub name: String,
    /// Task type
    pub task_type: TaskType,
    /// Command to run
    pub command: String,
    /// Arguments
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Whether to use shell
    pub shell: bool,
    /// Problem matcher to use
    pub problem_matcher: Option<String>,
    /// Whether to clear terminal before running
    pub clear: bool,
    /// Whether to reveal the terminal
    pub reveal: bool,
    /// Whether to close on success
    pub close_on_success: bool,
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            name: "unnamed".to_string(),
            task_type: TaskType::Custom,
            command: String::new(),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            shell: false,
            problem_matcher: Some("rust".to_string()),
            clear: true,
            reveal: true,
            close_on_success: false,
        }
    }
}

impl TaskConfig {
    /// Create a new build task
    pub fn build(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Build,
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
            problem_matcher: Some("rust".to_string()),
            ..Default::default()
        }
    }

    /// Create a new test task
    pub fn test(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Test,
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
            problem_matcher: Some("rust".to_string()),
            ..Default::default()
        }
    }

    /// Create a new check task
    pub fn check(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Check,
            command: "cargo".to_string(),
            args: vec!["check".to_string()],
            problem_matcher: Some("rust".to_string()),
            ..Default::default()
        }
    }

    /// Create a new run task
    pub fn run(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Run,
            command: "cargo".to_string(),
            args: vec!["run".to_string()],
            problem_matcher: Some("rust".to_string()),
            ..Default::default()
        }
    }

    /// Create a new clean task
    pub fn clean(name: &str) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Clean,
            command: "cargo".to_string(),
            args: vec!["clean".to_string()],
            problem_matcher: None,
            ..Default::default()
        }
    }

    /// Create a custom task
    pub fn custom(name: &str, command: &str, args: &[&str]) -> Self {
        Self {
            name: name.to_string(),
            task_type: TaskType::Custom,
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn with_cwd(mut self, cwd: PathBuf) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: &str, value: &str) -> Self {
        self.env.insert(key.to_string(), value.to_string());
        self
    }

    /// Add argument
    pub fn with_arg(mut self, arg: &str) -> Self {
        self.args.push(arg.to_string());
        self
    }
}

/// A running or completed task
#[derive(Debug)]
pub struct Task {
    /// Task ID
    pub id: u64,
    /// Task configuration
    pub config: TaskConfig,
    /// Current status
    pub status: TaskStatus,
    /// Collected output lines
    pub output: Vec<OutputLine>,
    /// Problems found
    pub problems: Vec<Problem>,
    /// Exit code if completed
    pub exit_code: Option<i32>,
}

impl Task {
    fn new(id: u64, config: TaskConfig) -> Self {
        Self {
            id,
            config,
            status: TaskStatus::Pending,
            output: Vec::new(),
            problems: Vec::new(),
            exit_code: None,
        }
    }
}

/// Task runner manages and executes tasks
pub struct TaskRunner {
    /// All registered tasks
    tasks: Arc<RwLock<HashMap<u64, Task>>>,
    /// Running processes
    processes: Arc<RwLock<HashMap<u64, Child>>>,
    /// Cancel channels for running tasks
    cancel_txs: Arc<RwLock<HashMap<u64, oneshot::Sender<()>>>>,
    /// Next task ID
    next_id: Arc<RwLock<u64>>,
    /// Output callback
    output_callback: Option<Arc<dyn Fn(u64, OutputLine) + Send + Sync>>,
    /// Problem callback
    problem_callback: Option<Arc<dyn Fn(u64, Problem) + Send + Sync>>,
    /// Status callback
    status_callback: Option<Arc<dyn Fn(u64, TaskStatus) + Send + Sync>>,
}

impl TaskRunner {
    /// Create a new task runner
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(RwLock::new(HashMap::new())),
            cancel_txs: Arc::new(RwLock::new(HashMap::new())),
            next_id: Arc::new(RwLock::new(1)),
            output_callback: None,
            problem_callback: None,
            status_callback: None,
        }
    }

    /// Set output callback
    pub fn set_output_callback<F>(&mut self, callback: F)
    where
        F: Fn(u64, OutputLine) + Send + Sync + 'static,
    {
        self.output_callback = Some(Arc::new(callback));
    }

    /// Set problem callback
    pub fn set_problem_callback<F>(&mut self, callback: F)
    where
        F: Fn(u64, Problem) + Send + Sync + 'static,
    {
        self.problem_callback = Some(Arc::new(callback));
    }

    /// Set status callback
    pub fn set_status_callback<F>(&mut self, callback: F)
    where
        F: Fn(u64, TaskStatus) + Send + Sync + 'static,
    {
        self.status_callback = Some(Arc::new(callback));
    }

    /// Run a task
    pub async fn run(&self, config: TaskConfig) -> Result<u64, TaskError> {
        let id = {
            let mut next_id = self.next_id.write().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        info!("Running task {}: {}", id, config.name);

        let task = Task::new(id, config.clone());

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(id, task);
        }

        // Build command
        let mut cmd = if config.shell {
            let shell = if cfg!(target_os = "windows") {
                "cmd"
            } else {
                "sh"
            };
            let shell_arg = if cfg!(target_os = "windows") {
                "/C"
            } else {
                "-c"
            };
            let full_cmd = format!("{} {}", config.command, config.args.join(" "));

            let mut cmd = Command::new(shell);
            cmd.arg(shell_arg).arg(full_cmd);
            cmd
        } else {
            let mut cmd = Command::new(&config.command);
            cmd.args(&config.args);
            cmd
        };

        // Set working directory
        if let Some(ref cwd) = config.cwd {
            cmd.current_dir(cwd);
        }

        // Set environment variables
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Spawn process
        let mut child = cmd.spawn().map_err(|e| TaskError::SpawnError(e.to_string()))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        // Store process
        {
            let mut processes = self.processes.write().await;
            processes.insert(id, child);
        }

        // Create cancel channel
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut cancel_txs = self.cancel_txs.write().await;
            cancel_txs.insert(id, cancel_tx);
        }

        // Update status
        self.update_status(id, TaskStatus::Running).await;

        // Spawn output handlers
        let tasks = self.tasks.clone();
        let processes = self.processes.clone();
        let output_callback = self.output_callback.clone();
        let problem_callback = self.problem_callback.clone();
        let status_callback = self.status_callback.clone();
        let problem_matcher: Box<dyn ProblemMatcher + Send + Sync> =
            Box::new(RustProblemMatcher::new());

        tokio::spawn(async move {
            let mut output_lines = Vec::new();
            let mut problems = Vec::new();

            // Handle stdout
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let output_line = OutputLine::new(OutputType::Stdout, &line);
                    output_lines.push(output_line.clone());

                    // Check for problems
                    if let Some(problem) = problem_matcher.match_line(&line) {
                        problems.push(problem.clone());
                        if let Some(ref cb) = problem_callback {
                            cb(id, problem);
                        }
                    }

                    if let Some(ref cb) = output_callback {
                        cb(id, output_line);
                    }
                }
            }

            // Handle stderr
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    let output_line = OutputLine::new(OutputType::Stderr, &line);
                    output_lines.push(output_line.clone());

                    // Check for problems
                    if let Some(problem) = problem_matcher.match_line(&line) {
                        problems.push(problem.clone());
                        if let Some(ref cb) = problem_callback {
                            cb(id, problem);
                        }
                    }

                    if let Some(ref cb) = output_callback {
                        cb(id, output_line);
                    }
                }
            }

            // Wait for process to finish
            let exit_status = {
                let mut processes = processes.write().await;
                if let Some(mut child) = processes.remove(&id) {
                    child.wait().await.ok()
                } else {
                    None
                }
            };

            // Determine final status
            let (status, exit_code) = match exit_status {
                Some(status) => {
                    let code = status.code().unwrap_or(-1);
                    let task_status = if status.success() {
                        TaskStatus::Success
                    } else {
                        TaskStatus::Failed(code)
                    };
                    (task_status, Some(code))
                }
                None => (TaskStatus::Cancelled, None),
            };

            // Update task
            {
                let mut tasks = tasks.write().await;
                if let Some(task) = tasks.get_mut(&id) {
                    task.status = status.clone();
                    task.exit_code = exit_code;
                    task.output = output_lines;
                    task.problems = problems;
                }
            }

            // Notify status change
            if let Some(ref cb) = status_callback {
                cb(id, status);
            }
        });

        Ok(id)
    }

    /// Cancel a running task
    pub async fn cancel(&self, id: u64) -> Result<(), TaskError> {
        info!("Cancelling task {}", id);

        // Send cancel signal
        {
            let mut cancel_txs = self.cancel_txs.write().await;
            if let Some(tx) = cancel_txs.remove(&id) {
                let _ = tx.send(());
            }
        }

        // Kill process
        {
            let mut processes = self.processes.write().await;
            if let Some(mut child) = processes.remove(&id) {
                let _ = child.kill().await;
            }
        }

        // Update status
        self.update_status(id, TaskStatus::Cancelled).await;

        Ok(())
    }

    /// Get task by ID
    pub async fn get(&self, id: u64) -> Option<Task> {
        // We can't clone Task easily, so we'll return key info
        let tasks = self.tasks.read().await;
        tasks.get(&id).map(|t| Task {
            id: t.id,
            config: t.config.clone(),
            status: t.status.clone(),
            output: t.output.clone(),
            problems: t.problems.clone(),
            exit_code: t.exit_code,
        })
    }

    /// Get task status
    pub async fn status(&self, id: u64) -> Option<TaskStatus> {
        let tasks = self.tasks.read().await;
        tasks.get(&id).map(|t| t.status.clone())
    }

    /// Get all tasks
    pub async fn all_tasks(&self) -> Vec<(u64, TaskConfig, TaskStatus)> {
        let tasks = self.tasks.read().await;
        tasks
            .iter()
            .map(|(id, t)| (*id, t.config.clone(), t.status.clone()))
            .collect()
    }

    /// Get running tasks
    pub async fn running_tasks(&self) -> Vec<u64> {
        let tasks = self.tasks.read().await;
        tasks
            .iter()
            .filter(|(_, t)| matches!(t.status, TaskStatus::Running))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Update task status
    async fn update_status(&self, id: u64, status: TaskStatus) {
        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&id) {
                task.status = status.clone();
            }
        }

        if let Some(ref cb) = self.status_callback {
            cb(id, status);
        }
    }

    /// Clear completed tasks
    pub async fn clear_completed(&self) {
        let mut tasks = self.tasks.write().await;
        tasks.retain(|_, t| matches!(t.status, TaskStatus::Running | TaskStatus::Pending));
    }

    /// Remove a task
    pub async fn remove(&self, id: u64) -> Option<Task> {
        let mut tasks = self.tasks.write().await;
        tasks.remove(&id).map(|t| Task {
            id: t.id,
            config: t.config,
            status: t.status,
            output: t.output,
            problems: t.problems,
            exit_code: t.exit_code,
        })
    }
}

impl Default for TaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_config_build() {
        let config = TaskConfig::build("build");
        assert_eq!(config.command, "cargo");
        assert_eq!(config.args, vec!["build"]);
        assert_eq!(config.task_type, TaskType::Build);
    }

    #[test]
    fn test_task_config_test() {
        let config = TaskConfig::test("test");
        assert_eq!(config.command, "cargo");
        assert_eq!(config.args, vec!["test"]);
        assert_eq!(config.task_type, TaskType::Test);
    }

    #[test]
    fn test_task_config_custom() {
        let config = TaskConfig::custom("lint", "clippy", &["-D", "warnings"]);
        assert_eq!(config.command, "clippy");
        assert_eq!(config.args, vec!["-D", "warnings"]);
    }

    #[test]
    fn test_task_config_with_cwd() {
        let config = TaskConfig::build("build")
            .with_cwd(PathBuf::from("/project"));
        assert_eq!(config.cwd, Some(PathBuf::from("/project")));
    }

    #[test]
    fn test_task_config_with_env() {
        let config = TaskConfig::build("build")
            .with_env("RUST_BACKTRACE", "1");
        assert_eq!(config.env.get("RUST_BACKTRACE"), Some(&"1".to_string()));
    }

    #[tokio::test]
    async fn test_task_runner_creation() {
        let runner = TaskRunner::new();
        let tasks = runner.all_tasks().await;
        assert!(tasks.is_empty());
    }
}
