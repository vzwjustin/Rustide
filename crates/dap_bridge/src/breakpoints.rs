//! Breakpoint management

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};

/// Breakpoint state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreakpointState {
    /// Breakpoint is pending verification
    Pending,
    /// Breakpoint is verified and active
    Verified,
    /// Breakpoint is disabled
    Disabled,
    /// Breakpoint failed to set
    Failed(String),
}

/// A breakpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Unique ID for this breakpoint
    pub id: u64,
    /// Source file path
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (optional)
    pub column: Option<u32>,
    /// Condition expression
    pub condition: Option<String>,
    /// Hit count condition
    pub hit_condition: Option<String>,
    /// Log message instead of breaking
    pub log_message: Option<String>,
    /// Current state
    pub state: BreakpointState,
    /// Server-assigned ID
    pub server_id: Option<i64>,
}

impl Breakpoint {
    /// Create a new breakpoint
    pub fn new(id: u64, file: PathBuf, line: u32) -> Self {
        Self {
            id,
            file,
            line,
            column: None,
            condition: None,
            hit_condition: None,
            log_message: None,
            state: BreakpointState::Pending,
            server_id: None,
        }
    }

    /// Set a condition
    pub fn with_condition(mut self, condition: &str) -> Self {
        self.condition = Some(condition.to_string());
        self
    }

    /// Set a hit condition
    pub fn with_hit_condition(mut self, hit_condition: &str) -> Self {
        self.hit_condition = Some(hit_condition.to_string());
        self
    }

    /// Set a log message (logpoint)
    pub fn with_log_message(mut self, message: &str) -> Self {
        self.log_message = Some(message.to_string());
        self
    }

    /// Check if this is a logpoint
    pub fn is_logpoint(&self) -> bool {
        self.log_message.is_some()
    }

    /// Check if this breakpoint is active
    pub fn is_active(&self) -> bool {
        matches!(self.state, BreakpointState::Verified | BreakpointState::Pending)
    }
}

/// Manages breakpoints
pub struct BreakpointManager {
    /// All breakpoints indexed by ID
    breakpoints: RwLock<HashMap<u64, Breakpoint>>,
    /// Breakpoints indexed by file
    by_file: RwLock<HashMap<PathBuf, Vec<u64>>>,
    /// Next breakpoint ID
    next_id: AtomicU64,
    /// Dirty files that need to be synced
    dirty_files: RwLock<Vec<PathBuf>>,
}

impl BreakpointManager {
    /// Create a new breakpoint manager
    pub fn new() -> Self {
        Self {
            breakpoints: RwLock::new(HashMap::new()),
            by_file: RwLock::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            dirty_files: RwLock::new(Vec::new()),
        }
    }

    /// Generate the next breakpoint ID
    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Add a breakpoint
    pub fn add(&self, file: PathBuf, line: u32) -> Breakpoint {
        let id = self.next_id();
        let bp = Breakpoint::new(id, file.clone(), line);

        info!("Adding breakpoint {} at {}:{}", id, file.display(), line);

        {
            let mut breakpoints = self.breakpoints.write();
            breakpoints.insert(id, bp.clone());
        }

        {
            let mut by_file = self.by_file.write();
            by_file.entry(file.clone()).or_default().push(id);
        }

        self.mark_dirty(&file);

        bp
    }

    /// Add a breakpoint with condition
    pub fn add_conditional(&self, file: PathBuf, line: u32, condition: &str) -> Breakpoint {
        let id = self.next_id();
        let bp = Breakpoint::new(id, file.clone(), line).with_condition(condition);

        info!(
            "Adding conditional breakpoint {} at {}:{} with condition: {}",
            id,
            file.display(),
            line,
            condition
        );

        {
            let mut breakpoints = self.breakpoints.write();
            breakpoints.insert(id, bp.clone());
        }

        {
            let mut by_file = self.by_file.write();
            by_file.entry(file.clone()).or_default().push(id);
        }

        self.mark_dirty(&file);

        bp
    }

    /// Add a logpoint
    pub fn add_logpoint(&self, file: PathBuf, line: u32, message: &str) -> Breakpoint {
        let id = self.next_id();
        let bp = Breakpoint::new(id, file.clone(), line).with_log_message(message);

        info!(
            "Adding logpoint {} at {}:{} with message: {}",
            id,
            file.display(),
            line,
            message
        );

        {
            let mut breakpoints = self.breakpoints.write();
            breakpoints.insert(id, bp.clone());
        }

        {
            let mut by_file = self.by_file.write();
            by_file.entry(file.clone()).or_default().push(id);
        }

        self.mark_dirty(&file);

        bp
    }

    /// Remove a breakpoint by ID
    pub fn remove(&self, id: u64) -> Option<Breakpoint> {
        let bp = {
            let mut breakpoints = self.breakpoints.write();
            breakpoints.remove(&id)
        };

        if let Some(ref bp) = bp {
            info!("Removing breakpoint {} at {}:{}", id, bp.file.display(), bp.line);

            {
                let mut by_file = self.by_file.write();
                if let Some(ids) = by_file.get_mut(&bp.file) {
                    ids.retain(|&i| i != id);
                    if ids.is_empty() {
                        by_file.remove(&bp.file);
                    }
                }
            }

            self.mark_dirty(&bp.file);
        }

        bp
    }

    /// Remove all breakpoints in a file
    pub fn remove_file(&self, file: &PathBuf) -> Vec<Breakpoint> {
        let ids: Vec<u64> = {
            let by_file = self.by_file.read();
            by_file.get(file).cloned().unwrap_or_default()
        };

        let mut removed = Vec::new();

        for id in ids {
            if let Some(bp) = self.remove(id) {
                removed.push(bp);
            }
        }

        removed
    }

    /// Toggle a breakpoint at a location
    pub fn toggle(&self, file: PathBuf, line: u32) -> Option<Breakpoint> {
        // Check if there's a breakpoint at this location
        let existing_id = {
            let breakpoints = self.breakpoints.read();
            let by_file = self.by_file.read();

            by_file.get(&file).and_then(|ids| {
                ids.iter().find(|&&id| {
                    breakpoints.get(&id).map(|bp| bp.line == line).unwrap_or(false)
                }).copied()
            })
        };

        if let Some(id) = existing_id {
            // Remove existing breakpoint
            self.remove(id);
            None
        } else {
            // Add new breakpoint
            Some(self.add(file, line))
        }
    }

    /// Get a breakpoint by ID
    pub fn get(&self, id: u64) -> Option<Breakpoint> {
        self.breakpoints.read().get(&id).cloned()
    }

    /// Get all breakpoints for a file
    pub fn get_breakpoints_for_file(&self, file: &PathBuf) -> Vec<Breakpoint> {
        let by_file = self.by_file.read();
        let breakpoints = self.breakpoints.read();

        by_file
            .get(file)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| breakpoints.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all breakpoints
    pub fn all(&self) -> Vec<Breakpoint> {
        self.breakpoints.read().values().cloned().collect()
    }

    /// Get all files with breakpoints
    pub fn files_with_breakpoints(&self) -> Vec<PathBuf> {
        self.by_file.read().keys().cloned().collect()
    }

    /// Update breakpoint state
    pub fn update_state(&self, id: u64, state: BreakpointState) {
        let mut breakpoints = self.breakpoints.write();
        if let Some(bp) = breakpoints.get_mut(&id) {
            debug!("Updating breakpoint {} state to {:?}", id, state);
            bp.state = state;
        }
    }

    /// Set server ID for a breakpoint
    pub fn set_server_id(&self, id: u64, server_id: i64) {
        let mut breakpoints = self.breakpoints.write();
        if let Some(bp) = breakpoints.get_mut(&id) {
            bp.server_id = Some(server_id);
        }
    }

    /// Enable a breakpoint
    pub fn enable(&self, id: u64) {
        let file = {
            let mut breakpoints = self.breakpoints.write();
            if let Some(bp) = breakpoints.get_mut(&id) {
                if matches!(bp.state, BreakpointState::Disabled) {
                    bp.state = BreakpointState::Pending;
                    Some(bp.file.clone())
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(file) = file {
            self.mark_dirty(&file);
        }
    }

    /// Disable a breakpoint
    pub fn disable(&self, id: u64) {
        let file = {
            let mut breakpoints = self.breakpoints.write();
            if let Some(bp) = breakpoints.get_mut(&id) {
                bp.state = BreakpointState::Disabled;
                Some(bp.file.clone())
            } else {
                None
            }
        };

        if let Some(file) = file {
            self.mark_dirty(&file);
        }
    }

    /// Update breakpoint condition
    pub fn set_condition(&self, id: u64, condition: Option<&str>) {
        let file = {
            let mut breakpoints = self.breakpoints.write();
            if let Some(bp) = breakpoints.get_mut(&id) {
                bp.condition = condition.map(|s| s.to_string());
                Some(bp.file.clone())
            } else {
                None
            }
        };

        if let Some(file) = file {
            self.mark_dirty(&file);
        }
    }

    /// Mark a file as dirty (needs sync)
    fn mark_dirty(&self, file: &PathBuf) {
        let mut dirty = self.dirty_files.write();
        if !dirty.contains(file) {
            dirty.push(file.clone());
        }
    }

    /// Get and clear dirty files
    pub fn take_dirty_files(&self) -> Vec<PathBuf> {
        let mut dirty = self.dirty_files.write();
        std::mem::take(&mut *dirty)
    }

    /// Clear all breakpoints
    pub fn clear(&self) {
        let files: Vec<PathBuf> = self.by_file.read().keys().cloned().collect();

        self.breakpoints.write().clear();
        self.by_file.write().clear();

        let mut dirty = self.dirty_files.write();
        dirty.extend(files);
    }

    /// Get breakpoint count
    pub fn count(&self) -> usize {
        self.breakpoints.read().len()
    }
}

impl Default for BreakpointManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_breakpoint() {
        let manager = BreakpointManager::new();
        let bp = manager.add(PathBuf::from("/test/file.rs"), 10);

        assert_eq!(bp.line, 10);
        assert_eq!(bp.file, PathBuf::from("/test/file.rs"));
        assert!(matches!(bp.state, BreakpointState::Pending));
    }

    #[test]
    fn test_remove_breakpoint() {
        let manager = BreakpointManager::new();
        let bp = manager.add(PathBuf::from("/test/file.rs"), 10);

        assert_eq!(manager.count(), 1);
        manager.remove(bp.id);
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_toggle_breakpoint() {
        let manager = BreakpointManager::new();
        let file = PathBuf::from("/test/file.rs");

        // Add
        let bp = manager.toggle(file.clone(), 10);
        assert!(bp.is_some());
        assert_eq!(manager.count(), 1);

        // Remove
        let bp = manager.toggle(file.clone(), 10);
        assert!(bp.is_none());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_conditional_breakpoint() {
        let manager = BreakpointManager::new();
        let bp = manager.add_conditional(
            PathBuf::from("/test/file.rs"),
            10,
            "x > 5",
        );

        assert_eq!(bp.condition, Some("x > 5".to_string()));
    }

    #[test]
    fn test_logpoint() {
        let manager = BreakpointManager::new();
        let bp = manager.add_logpoint(
            PathBuf::from("/test/file.rs"),
            10,
            "Value: {x}",
        );

        assert!(bp.is_logpoint());
        assert_eq!(bp.log_message, Some("Value: {x}".to_string()));
    }

    #[test]
    fn test_get_breakpoints_for_file() {
        let manager = BreakpointManager::new();
        let file = PathBuf::from("/test/file.rs");

        manager.add(file.clone(), 10);
        manager.add(file.clone(), 20);
        manager.add(PathBuf::from("/test/other.rs"), 5);

        let bps = manager.get_breakpoints_for_file(&file);
        assert_eq!(bps.len(), 2);
    }

    #[test]
    fn test_dirty_files() {
        let manager = BreakpointManager::new();
        let file = PathBuf::from("/test/file.rs");

        manager.add(file.clone(), 10);

        let dirty = manager.take_dirty_files();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0], file);

        // Should be empty after taking
        let dirty = manager.take_dirty_files();
        assert!(dirty.is_empty());
    }
}
