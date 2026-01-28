//! DAP Bridge - Debug Adapter Protocol client for Rust IDE
//!
//! This crate provides a client implementation for the Debug Adapter Protocol (DAP),
//! enabling debugging capabilities in the IDE.

pub mod breakpoints;
pub mod client;
pub mod session;

pub use breakpoints::{Breakpoint, BreakpointManager, BreakpointState};
pub use client::{DapClient, DapClientConfig, DapError};
pub use session::{DebugSession, SessionState, StoppedReason};
