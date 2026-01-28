//! RustIDE - Binary entry point
//!
//! This is the main entry point for the RustIDE application.

use anyhow::Result;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use ui_shell::RustideApp;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("ui_shell=debug".parse()?))
        .init();

    tracing::info!("Starting RustIDE");

    // Create and run the application
    let app = RustideApp::new();
    app.run()?;

    tracing::info!("RustIDE shutting down");
    Ok(())
}
