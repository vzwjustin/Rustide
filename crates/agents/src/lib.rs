//! Agents - Context-aware agent system for Rust IDE
//!
//! This crate provides an agent framework with context capsules for
//! managing state, tools for agent interactions, and orchestration
//! for coordinating multiple agents.

pub mod audit;
pub mod capsule;
pub mod orchestrator;
pub mod tools;

pub use audit::{AuditEntry, AuditLog, AuditLevel};
pub use capsule::{ContextCapsule, CapsuleBuilder, CapsuleField};
pub use orchestrator::{Agent, AgentOrchestrator, AgentState, AgentTask};
pub use tools::{Tool, ToolCall, ToolRegistry, ToolResult};
