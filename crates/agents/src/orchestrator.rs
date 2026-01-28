//! Agent orchestration - Coordinates multiple agents

use crate::audit::{AuditEntry, AuditLevel, AuditLog};
use crate::capsule::{CapsuleStore, ContextCapsule};
use crate::tools::{ToolCall, ToolRegistry, ToolResult};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Errors that can occur in orchestration
#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Agent not found: {0}")]
    AgentNotFound(Uuid),

    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("Agent busy")]
    AgentBusy,

    #[error("Task failed: {0}")]
    TaskFailed(String),

    #[error("Communication error: {0}")]
    CommunicationError(String),
}

/// State of an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is idle
    Idle,
    /// Agent is processing a task
    Processing,
    /// Agent is waiting for input
    Waiting,
    /// Agent has completed
    Completed,
    /// Agent encountered an error
    Error(String),
    /// Agent was terminated
    Terminated,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Idle
    }
}

/// A task for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    /// Task ID
    pub id: Uuid,
    /// Task description
    pub description: String,
    /// Task input
    pub input: Value,
    /// Task output (when completed)
    pub output: Option<Value>,
    /// Priority (higher = more important)
    pub priority: i32,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Parent task (if subtask)
    pub parent_id: Option<Uuid>,
    /// Whether task is complete
    pub complete: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl AgentTask {
    /// Create a new task
    pub fn new(description: &str, input: Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.to_string(),
            input,
            output: None,
            priority: 0,
            created_at: Utc::now(),
            completed_at: None,
            parent_id: None,
            complete: false,
            error: None,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set parent task
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Mark as complete
    pub fn complete(&mut self, output: Value) {
        self.output = Some(output);
        self.complete = true;
        self.completed_at = Some(Utc::now());
    }

    /// Mark as failed
    pub fn fail(&mut self, error: &str) {
        self.error = Some(error.to_string());
        self.complete = true;
        self.completed_at = Some(Utc::now());
    }

    /// Check if task succeeded
    pub fn succeeded(&self) -> bool {
        self.complete && self.error.is_none()
    }
}

/// An agent that can process tasks
#[derive(Debug)]
pub struct Agent {
    /// Agent ID
    pub id: Uuid,
    /// Agent name
    pub name: String,
    /// Agent description
    pub description: String,
    /// Current state
    pub state: AgentState,
    /// Current task
    pub current_task: Option<Uuid>,
    /// Completed tasks
    pub completed_tasks: Vec<Uuid>,
    /// Context capsule
    pub context: Option<Uuid>,
    /// Available tools
    pub tools: Vec<String>,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Permissions
    pub permissions: Vec<String>,
}

impl Agent {
    /// Create a new agent
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            state: AgentState::Idle,
            current_task: None,
            completed_tasks: Vec::new(),
            context: None,
            tools: Vec::new(),
            created_at: Utc::now(),
            permissions: Vec::new(),
        }
    }

    /// Add a tool to this agent
    pub fn with_tool(mut self, tool: &str) -> Self {
        self.tools.push(tool.to_string());
        self
    }

    /// Add permission
    pub fn with_permission(mut self, permission: &str) -> Self {
        self.permissions.push(permission.to_string());
        self
    }

    /// Set context capsule
    pub fn with_context(mut self, context_id: Uuid) -> Self {
        self.context = Some(context_id);
        self
    }

    /// Check if agent can use a tool
    pub fn can_use_tool(&self, tool: &str) -> bool {
        self.tools.contains(&tool.to_string()) || self.tools.contains(&"*".to_string())
    }

    /// Check if agent has permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
            || self.permissions.contains(&"*".to_string())
    }

    /// Check if agent is available
    pub fn is_available(&self) -> bool {
        matches!(self.state, AgentState::Idle)
    }
}

/// Message between orchestrator and agents
#[derive(Debug, Clone)]
pub enum AgentMessage {
    /// Assign a task
    AssignTask(AgentTask),
    /// Cancel current task
    Cancel,
    /// Pause agent
    Pause,
    /// Resume agent
    Resume,
    /// Terminate agent
    Terminate,
    /// Tool call result
    ToolResult(ToolResult),
}

/// Response from an agent
#[derive(Debug, Clone)]
pub enum AgentResponse {
    /// Task started
    TaskStarted(Uuid),
    /// Task completed
    TaskCompleted(Uuid, Value),
    /// Task failed
    TaskFailed(Uuid, String),
    /// Tool call requested
    ToolRequest(ToolCall),
    /// Status update
    StatusUpdate(AgentState),
}

/// The agent orchestrator
pub struct AgentOrchestrator {
    /// All agents
    agents: RwLock<HashMap<Uuid, Agent>>,
    /// All tasks
    tasks: RwLock<HashMap<Uuid, AgentTask>>,
    /// Tool registry
    tools: Arc<ToolRegistry>,
    /// Capsule store
    capsules: Arc<CapsuleStore>,
    /// Audit log
    audit: Arc<AuditLog>,
    /// Task queue
    task_queue: RwLock<Vec<Uuid>>,
}

impl AgentOrchestrator {
    /// Create a new orchestrator
    pub fn new(tools: Arc<ToolRegistry>, capsules: Arc<CapsuleStore>) -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            tools,
            capsules,
            audit: Arc::new(AuditLog::new()),
            task_queue: RwLock::new(Vec::new()),
        }
    }

    /// Register an agent
    pub fn register_agent(&self, agent: Agent) -> Uuid {
        let id = agent.id;
        info!("Registering agent: {} ({})", agent.name, id);

        self.audit.log(AuditEntry::new(
            AuditLevel::Info,
            "agent_registered",
            &format!("Agent {} registered", agent.name),
        ));

        self.agents.write().insert(id, agent);
        id
    }

    /// Get an agent
    pub fn get_agent(&self, id: &Uuid) -> Option<Agent> {
        self.agents.read().get(id).map(|a| Agent {
            id: a.id,
            name: a.name.clone(),
            description: a.description.clone(),
            state: a.state.clone(),
            current_task: a.current_task,
            completed_tasks: a.completed_tasks.clone(),
            context: a.context,
            tools: a.tools.clone(),
            created_at: a.created_at,
            permissions: a.permissions.clone(),
        })
    }

    /// Remove an agent
    pub fn remove_agent(&self, id: &Uuid) -> Option<Agent> {
        let agent = self.agents.write().remove(id);
        if let Some(ref a) = agent {
            self.audit.log(AuditEntry::new(
                AuditLevel::Info,
                "agent_removed",
                &format!("Agent {} removed", a.name),
            ));
        }
        agent
    }

    /// List all agents
    pub fn list_agents(&self) -> Vec<(Uuid, String, AgentState)> {
        self.agents
            .read()
            .values()
            .map(|a| (a.id, a.name.clone(), a.state.clone()))
            .collect()
    }

    /// Submit a task
    pub fn submit_task(&self, task: AgentTask) -> Uuid {
        let id = task.id;
        info!("Task submitted: {} ({})", task.description, id);

        self.audit.log(AuditEntry::new(
            AuditLevel::Info,
            "task_submitted",
            &format!("Task {} submitted", task.description),
        ));

        self.tasks.write().insert(id, task);
        self.task_queue.write().push(id);
        id
    }

    /// Get a task
    pub fn get_task(&self, id: &Uuid) -> Option<AgentTask> {
        self.tasks.read().get(id).cloned()
    }

    /// Assign task to an agent
    pub fn assign_task(&self, task_id: Uuid, agent_id: Uuid) -> Result<(), OrchestratorError> {
        let mut agents = self.agents.write();
        let agent = agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        if !agent.is_available() {
            return Err(OrchestratorError::AgentBusy);
        }

        agent.state = AgentState::Processing;
        agent.current_task = Some(task_id);

        self.audit.log(AuditEntry::new(
            AuditLevel::Info,
            "task_assigned",
            &format!("Task {} assigned to agent {}", task_id, agent.name),
        ));

        // Remove from queue
        self.task_queue.write().retain(|&id| id != task_id);

        Ok(())
    }

    /// Complete a task
    pub fn complete_task(
        &self,
        task_id: Uuid,
        output: Value,
    ) -> Result<(), OrchestratorError> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        task.complete(output);

        // Find and update the agent
        let mut agents = self.agents.write();
        for agent in agents.values_mut() {
            if agent.current_task == Some(task_id) {
                agent.current_task = None;
                agent.completed_tasks.push(task_id);
                agent.state = AgentState::Idle;
                break;
            }
        }

        self.audit.log(AuditEntry::new(
            AuditLevel::Info,
            "task_completed",
            &format!("Task {} completed", task_id),
        ));

        Ok(())
    }

    /// Fail a task
    pub fn fail_task(&self, task_id: Uuid, error: &str) -> Result<(), OrchestratorError> {
        let mut tasks = self.tasks.write();
        let task = tasks
            .get_mut(&task_id)
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        task.fail(error);

        // Find and update the agent
        let mut agents = self.agents.write();
        for agent in agents.values_mut() {
            if agent.current_task == Some(task_id) {
                agent.current_task = None;
                agent.state = AgentState::Error(error.to_string());
                break;
            }
        }

        self.audit.log(AuditEntry::new(
            AuditLevel::Error,
            "task_failed",
            &format!("Task {} failed: {}", task_id, error),
        ));

        Ok(())
    }

    /// Execute a tool call for an agent
    pub async fn execute_tool(
        &self,
        agent_id: Uuid,
        call: ToolCall,
    ) -> Result<ToolResult, OrchestratorError> {
        // Check agent permissions
        let agent = self
            .get_agent(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        if !agent.can_use_tool(&call.tool) {
            return Ok(ToolResult::failure(&format!(
                "Agent does not have access to tool: {}",
                call.tool
            )));
        }

        // Execute the tool
        let result = self
            .tools
            .execute(call.clone())
            .await
            .map_err(|e| OrchestratorError::TaskFailed(e.to_string()))?;

        self.audit.log(AuditEntry::new(
            if result.success {
                AuditLevel::Debug
            } else {
                AuditLevel::Warning
            },
            "tool_executed",
            &format!("Tool {} executed by agent {}", call.tool, agent.name),
        ));

        Ok(result)
    }

    /// Get pending tasks
    pub fn pending_tasks(&self) -> Vec<AgentTask> {
        let queue = self.task_queue.read();
        let tasks = self.tasks.read();

        queue
            .iter()
            .filter_map(|id| tasks.get(id).cloned())
            .collect()
    }

    /// Get available agents
    pub fn available_agents(&self) -> Vec<Uuid> {
        self.agents
            .read()
            .values()
            .filter(|a| a.is_available())
            .map(|a| a.id)
            .collect()
    }

    /// Auto-assign pending tasks to available agents
    pub fn auto_assign(&self) -> Vec<(Uuid, Uuid)> {
        let available = self.available_agents();
        let pending: Vec<Uuid> = self.task_queue.read().clone();

        let mut assignments = Vec::new();

        for (task_id, agent_id) in pending.iter().zip(available.iter()) {
            if self.assign_task(*task_id, *agent_id).is_ok() {
                assignments.push((*task_id, *agent_id));
            }
        }

        assignments
    }

    /// Get the audit log
    pub fn audit_log(&self) -> Arc<AuditLog> {
        self.audit.clone()
    }

    /// Get statistics
    pub fn stats(&self) -> OrchestratorStats {
        let agents = self.agents.read();
        let tasks = self.tasks.read();

        OrchestratorStats {
            total_agents: agents.len(),
            idle_agents: agents.values().filter(|a| a.is_available()).count(),
            total_tasks: tasks.len(),
            pending_tasks: self.task_queue.read().len(),
            completed_tasks: tasks.values().filter(|t| t.succeeded()).count(),
            failed_tasks: tasks.values().filter(|t| t.error.is_some()).count(),
        }
    }
}

/// Orchestrator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub total_agents: usize,
    pub idle_agents: usize,
    pub total_tasks: usize,
    pub pending_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new(
            Arc::new(ToolRegistry::new()),
            Arc::new(CapsuleStore::new()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("test_agent", "A test agent")
            .with_tool("read_file")
            .with_permission("file:read");

        assert_eq!(agent.name, "test_agent");
        assert!(agent.can_use_tool("read_file"));
        assert!(agent.has_permission("file:read"));
        assert!(agent.is_available());
    }

    #[test]
    fn test_task_creation() {
        let task = AgentTask::new("Test task", serde_json::json!({"input": "test"}))
            .with_priority(10);

        assert_eq!(task.description, "Test task");
        assert_eq!(task.priority, 10);
        assert!(!task.complete);
    }

    #[test]
    fn test_task_completion() {
        let mut task = AgentTask::new("Test", serde_json::json!({}));
        task.complete(serde_json::json!({"result": "done"}));

        assert!(task.complete);
        assert!(task.succeeded());
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_failure() {
        let mut task = AgentTask::new("Test", serde_json::json!({}));
        task.fail("Something went wrong");

        assert!(task.complete);
        assert!(!task.succeeded());
        assert!(task.error.is_some());
    }

    #[test]
    fn test_orchestrator() {
        let orchestrator = AgentOrchestrator::default();

        let agent = Agent::new("agent1", "First agent");
        let agent_id = orchestrator.register_agent(agent);

        let task = AgentTask::new("Task 1", serde_json::json!({}));
        let task_id = orchestrator.submit_task(task);

        assert!(orchestrator.assign_task(task_id, agent_id).is_ok());

        let stats = orchestrator.stats();
        assert_eq!(stats.total_agents, 1);
        assert_eq!(stats.total_tasks, 1);
    }
}
