//! LSP Bridge - Language Server Protocol client and manager.
//!
//! This crate provides a robust implementation for communicating with
//! Language Server Protocol (LSP) servers, enabling IDE features such as:
//!
//! - Code completion
//! - Go to definition
//! - Find references
//! - Hover information
//! - Diagnostics
//! - Code actions
//! - Formatting
//! - And more
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`client`]: Core LSP client for spawning servers and handling JSON-RPC messages
//! - [`manager`]: Server lifecycle management for multiple language servers
//! - [`protocol`]: JSON-RPC protocol types and LSP message wrappers
//! - [`capabilities`]: Server capability parsing and feature detection
//! - [`diagnostics`]: Diagnostic storage and querying
//!
//! # Example
//!
//! ```rust,no_run
//! use lsp_bridge::{ServerManager, ServerManagerBuilder, LspServerConfig, LanguageServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a server manager with pre-configured servers
//!     let manager = ServerManagerBuilder::new("/path/to/workspace")
//!         .with_rust_analyzer()
//!         .with_typescript_lsp()
//!         .build();
//!
//!     // Start the Rust language server
//!     manager.start_server("rust").await?;
//!
//!     // The server is now ready to handle requests
//!     assert!(manager.is_server_running("rust"));
//!
//!     Ok(())
//! }
//! ```

pub mod capabilities;
pub mod client;
pub mod diagnostics;
pub mod manager;
pub mod protocol;

// Re-export main types for convenience
pub use capabilities::{FeatureSummary, ServerCapabilityInfo};
pub use client::{ClientState, LspClient, LspClientError, LspServerConfig};
pub use diagnostics::{DiagnosticStore, DiagnosticSummary};
pub use manager::{LanguageServerConfig, ManagedServer, ManagerError, ServerManager, ServerManagerBuilder};
pub use protocol::{
    JsonRpcError, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    LspMessage, RequestId,
};

// Re-export lsp-types for convenience
pub use lsp_types;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exports() {
        // Verify that main types are exported
        let _ = std::any::TypeId::of::<LspClient>();
        let _ = std::any::TypeId::of::<ServerManager>();
        let _ = std::any::TypeId::of::<DiagnosticStore>();
        let _ = std::any::TypeId::of::<ServerCapabilityInfo>();
    }
}
