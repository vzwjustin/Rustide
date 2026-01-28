//! UI Shell - GPUI-based application shell for RustIDE
//!
//! This crate provides the main application window, workspace layout,
//! command palette, file tree, and other UI components.

pub mod app;
pub mod bottom_panel;
pub mod command_palette;
pub mod editor_pane;
pub mod file_tree;
pub mod theme;
pub mod workspace;

pub use app::RustideApp;
pub use bottom_panel::BottomPanel;
pub use command_palette::CommandPalette;
pub use editor_pane::EditorPane;
pub use file_tree::FileTree;
pub use theme::Theme;
pub use workspace::Workspace;
