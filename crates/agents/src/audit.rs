//! Audit logging for agent activities

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use uuid::Uuid;

/// Audit level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Debug information
    Debug,
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical
    Critical,
}

impl Default for AuditLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl std::fmt::Display for AuditLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditLevel::Debug => write!(f, "DEBUG"),
            AuditLevel::Info => write!(f, "INFO"),
            AuditLevel::Warning => write!(f, "WARNING"),
            AuditLevel::Error => write!(f, "ERROR"),
            AuditLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// An audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub id: Uuid,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Level
    pub level: AuditLevel,
    /// Event type
    pub event_type: String,
    /// Event message
    pub message: String,
    /// Associated agent ID
    pub agent_id: Option<Uuid>,
    /// Associated task ID
    pub task_id: Option<Uuid>,
    /// Additional data
    pub data: Option<Value>,
    /// Source (file:line)
    pub source: Option<String>,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(level: AuditLevel, event_type: &str, message: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            level,
            event_type: event_type.to_string(),
            message: message.to_string(),
            agent_id: None,
            task_id: None,
            data: None,
            source: None,
        }
    }

    /// Create a debug entry
    pub fn debug(event_type: &str, message: &str) -> Self {
        Self::new(AuditLevel::Debug, event_type, message)
    }

    /// Create an info entry
    pub fn info(event_type: &str, message: &str) -> Self {
        Self::new(AuditLevel::Info, event_type, message)
    }

    /// Create a warning entry
    pub fn warning(event_type: &str, message: &str) -> Self {
        Self::new(AuditLevel::Warning, event_type, message)
    }

    /// Create an error entry
    pub fn error(event_type: &str, message: &str) -> Self {
        Self::new(AuditLevel::Error, event_type, message)
    }

    /// Create a critical entry
    pub fn critical(event_type: &str, message: &str) -> Self {
        Self::new(AuditLevel::Critical, event_type, message)
    }

    /// Set agent ID
    pub fn with_agent(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Set task ID
    pub fn with_task(mut self, task_id: Uuid) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Set additional data
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Set source location
    pub fn with_source(mut self, source: &str) -> Self {
        self.source = Some(source.to_string());
        self
    }

    /// Format as a log line
    pub fn format(&self) -> String {
        let mut parts = vec![
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
            format!("[{}]", self.level),
            format!("[{}]", self.event_type),
            self.message.clone(),
        ];

        if let Some(agent_id) = &self.agent_id {
            parts.push(format!("agent={}", agent_id));
        }
        if let Some(task_id) = &self.task_id {
            parts.push(format!("task={}", task_id));
        }

        parts.join(" ")
    }
}

/// Configuration for the audit log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Maximum number of entries to keep
    pub max_entries: usize,
    /// Minimum level to log
    pub min_level: AuditLevel,
    /// Whether to include debug entries
    pub include_debug: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            min_level: AuditLevel::Info,
            include_debug: false,
        }
    }
}

/// The audit log
pub struct AuditLog {
    /// Log entries
    entries: RwLock<VecDeque<AuditEntry>>,
    /// Configuration
    config: RwLock<AuditConfig>,
}

impl AuditLog {
    /// Create a new audit log
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            config: RwLock::new(AuditConfig::default()),
        }
    }

    /// Create with configuration
    pub fn with_config(config: AuditConfig) -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            config: RwLock::new(config),
        }
    }

    /// Log an entry
    pub fn log(&self, entry: AuditEntry) {
        let config = self.config.read();

        // Check if we should log this level
        if entry.level < config.min_level {
            return;
        }
        if entry.level == AuditLevel::Debug && !config.include_debug {
            return;
        }

        let mut entries = self.entries.write();

        // Enforce max entries
        while entries.len() >= config.max_entries {
            entries.pop_front();
        }

        entries.push_back(entry);
    }

    /// Get all entries
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries.read().iter().cloned().collect()
    }

    /// Get entries by level
    pub fn entries_by_level(&self, level: AuditLevel) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.level == level)
            .cloned()
            .collect()
    }

    /// Get entries by event type
    pub fn entries_by_type(&self, event_type: &str) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect()
    }

    /// Get entries for an agent
    pub fn entries_for_agent(&self, agent_id: Uuid) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.agent_id == Some(agent_id))
            .cloned()
            .collect()
    }

    /// Get entries for a task
    pub fn entries_for_task(&self, task_id: Uuid) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.task_id == Some(task_id))
            .cloned()
            .collect()
    }

    /// Get entries within a time range
    pub fn entries_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<AuditEntry> {
        self.entries
            .read()
            .iter()
            .filter(|e| e.timestamp >= start && e.timestamp <= end)
            .cloned()
            .collect()
    }

    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<AuditEntry> {
        let entries = self.entries.read();
        entries.iter().rev().take(count).cloned().collect()
    }

    /// Search entries by message
    pub fn search(&self, query: &str) -> Vec<AuditEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .read()
            .iter()
            .filter(|e| {
                e.message.to_lowercase().contains(&query_lower)
                    || e.event_type.to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Get entry count
    pub fn count(&self) -> usize {
        self.entries.read().len()
    }

    /// Get count by level
    pub fn count_by_level(&self, level: AuditLevel) -> usize {
        self.entries
            .read()
            .iter()
            .filter(|e| e.level == level)
            .count()
    }

    /// Update configuration
    pub fn configure(&self, config: AuditConfig) {
        *self.config.write() = config;
    }

    /// Export entries as JSON
    pub fn export_json(&self) -> Value {
        serde_json::to_value(self.entries()).unwrap_or(Value::Array(Vec::new()))
    }

    /// Export entries as formatted text
    pub fn export_text(&self) -> String {
        self.entries
            .read()
            .iter()
            .map(|e| e.format())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for creating audit entries with source location
#[macro_export]
macro_rules! audit {
    ($log:expr, $level:expr, $event:expr, $msg:expr) => {
        $log.log(
            AuditEntry::new($level, $event, $msg)
                .with_source(&format!("{}:{}", file!(), line!()))
        )
    };
    ($log:expr, $level:expr, $event:expr, $msg:expr, $($key:ident = $value:expr),*) => {
        {
            let mut entry = AuditEntry::new($level, $event, $msg)
                .with_source(&format!("{}:{}", file!(), line!()));
            $(
                entry = entry.with_data(serde_json::json!({ stringify!($key): $value }));
            )*
            $log.log(entry)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry() {
        let entry = AuditEntry::info("test_event", "Test message")
            .with_agent(Uuid::new_v4())
            .with_data(serde_json::json!({"key": "value"}));

        assert_eq!(entry.level, AuditLevel::Info);
        assert_eq!(entry.event_type, "test_event");
        assert!(entry.agent_id.is_some());
        assert!(entry.data.is_some());
    }

    #[test]
    fn test_audit_entry_format() {
        let entry = AuditEntry::warning("warn_event", "Warning message");
        let formatted = entry.format();

        assert!(formatted.contains("[WARNING]"));
        assert!(formatted.contains("[warn_event]"));
        assert!(formatted.contains("Warning message"));
    }

    #[test]
    fn test_audit_log() {
        let log = AuditLog::new();

        log.log(AuditEntry::info("event1", "Message 1"));
        log.log(AuditEntry::warning("event2", "Message 2"));
        log.log(AuditEntry::error("event3", "Message 3"));

        assert_eq!(log.count(), 3);
        assert_eq!(log.count_by_level(AuditLevel::Warning), 1);
    }

    #[test]
    fn test_audit_log_filtering() {
        let log = AuditLog::new();

        let agent_id = Uuid::new_v4();
        log.log(AuditEntry::info("event", "Message 1").with_agent(agent_id));
        log.log(AuditEntry::info("event", "Message 2"));

        let agent_entries = log.entries_for_agent(agent_id);
        assert_eq!(agent_entries.len(), 1);
    }

    #[test]
    fn test_audit_log_max_entries() {
        let config = AuditConfig {
            max_entries: 5,
            ..Default::default()
        };
        let log = AuditLog::with_config(config);

        for i in 0..10 {
            log.log(AuditEntry::info("event", &format!("Message {}", i)));
        }

        assert_eq!(log.count(), 5);

        // Should have the most recent entries
        let entries = log.entries();
        assert!(entries[0].message.contains("5"));
    }

    #[test]
    fn test_audit_log_search() {
        let log = AuditLog::new();

        log.log(AuditEntry::info("event", "Hello world"));
        log.log(AuditEntry::info("event", "Goodbye"));

        let results = log.search("hello");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_audit_level_ordering() {
        assert!(AuditLevel::Debug < AuditLevel::Info);
        assert!(AuditLevel::Info < AuditLevel::Warning);
        assert!(AuditLevel::Warning < AuditLevel::Error);
        assert!(AuditLevel::Error < AuditLevel::Critical);
    }

    #[test]
    fn test_audit_log_export() {
        let log = AuditLog::new();
        log.log(AuditEntry::info("event", "Test"));

        let json = log.export_json();
        assert!(json.is_array());

        let text = log.export_text();
        assert!(text.contains("Test"));
    }
}
