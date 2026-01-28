//! Security - Security utilities for Rust IDE
//!
//! This crate provides security features including secret redaction
//! and path policy enforcement.

pub mod path_policy;
pub mod redaction;

pub use path_policy::{PathPolicy, PathPolicyBuilder, PathRule, Permission};
pub use redaction::{RedactionPattern, Redactor, SecretType};
