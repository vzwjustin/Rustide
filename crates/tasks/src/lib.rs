//! Tasks - Task runner for Rust IDE
//!
//! This crate provides functionality to run build tasks, tests, and other
//! commands, parsing their output for problems and diagnostics.

pub mod output;
pub mod problem_matcher;
pub mod runner;

pub use output::{OutputLine, OutputParser, OutputType};
pub use problem_matcher::{Problem, ProblemMatcher, ProblemSeverity, RustProblemMatcher};
pub use runner::{Task, TaskConfig, TaskRunner, TaskStatus};
