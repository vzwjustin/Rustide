//! GPUI Application struct and initialization
//!
//! This module provides the main application structure and startup logic.

use anyhow::Result;
use gpui::{
    actions, prelude::*, px, App, Application, Global, KeyBinding,
    Menu, MenuItem, SharedString, TitlebarOptions, WindowBounds,
    WindowKind, WindowOptions,
};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use crate::theme::{self, Theme};
use crate::workspace::Workspace;

// Define application actions
actions!(
    rustide,
    [
        Quit,
        NewFile,
        OpenFile,
        SaveFile,
        SaveFileAs,
        CloseFile,
        ToggleCommandPalette,
        ToggleFileTree,
        ToggleBottomPanel,
        SplitRight,
        SplitDown,
        FocusNextPane,
        FocusPreviousPane,
    ]
);

/// Global application state accessible throughout the app
pub struct AppState {
    /// Current workspace root path
    pub workspace_root: Option<PathBuf>,
    /// Application theme
    pub theme: Theme,
    /// Recently opened files
    pub recent_files: Vec<PathBuf>,
    /// Recently opened projects
    pub recent_projects: Vec<PathBuf>,
}

impl Global for AppState {}

impl Default for AppState {
    fn default() -> Self {
        Self {
            workspace_root: None,
            theme: Theme::dark(),
            recent_files: Vec::new(),
            recent_projects: Vec::new(),
        }
    }
}

/// Main application struct
pub struct RustideApp {
    /// Application state
    state: Arc<RwLock<AppState>>,
}

impl RustideApp {
    /// Create a new RustIDE application instance
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(AppState::default())),
        }
    }

    /// Run the application
    pub fn run(self) -> Result<()> {
        tracing::info!("Starting RustIDE application");

        Application::new().run(|cx| {
            // Set global theme
            theme::set_theme(Theme::dark());

            // Register key bindings
            Self::register_key_bindings(cx);

            // Set up application menu
            Self::setup_menu(cx);

            // Initialize global state
            cx.set_global(AppState::default());

            // Open main window
            Self::open_main_window(cx);
        });

        Ok(())
    }

    /// Register global key bindings
    fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            // File operations
            KeyBinding::new("cmd-n", NewFile, None),
            KeyBinding::new("cmd-o", OpenFile, None),
            KeyBinding::new("cmd-s", SaveFile, None),
            KeyBinding::new("cmd-shift-s", SaveFileAs, None),
            KeyBinding::new("cmd-w", CloseFile, None),
            KeyBinding::new("cmd-q", Quit, None),
            // UI toggles
            KeyBinding::new("cmd-shift-p", ToggleCommandPalette, None),
            KeyBinding::new("cmd-b", ToggleFileTree, None),
            KeyBinding::new("cmd-j", ToggleBottomPanel, None),
            // Pane navigation
            KeyBinding::new("cmd-\\", SplitRight, None),
            KeyBinding::new("cmd-shift-\\", SplitDown, None),
            KeyBinding::new("cmd-]", FocusNextPane, None),
            KeyBinding::new("cmd-[", FocusPreviousPane, None),
        ]);
    }

    /// Set up the application menu bar
    fn setup_menu(cx: &mut App) {
        cx.set_menus(vec![
            Menu {
                name: SharedString::from("RustIDE"),
                items: vec![
                    MenuItem::action("About RustIDE", Quit),
                    MenuItem::separator(),
                    MenuItem::action("Quit", Quit),
                ],
            },
            Menu {
                name: SharedString::from("File"),
                items: vec![
                    MenuItem::action("New File", NewFile),
                    MenuItem::action("Open...", OpenFile),
                    MenuItem::separator(),
                    MenuItem::action("Save", SaveFile),
                    MenuItem::action("Save As...", SaveFileAs),
                    MenuItem::separator(),
                    MenuItem::action("Close", CloseFile),
                ],
            },
            Menu {
                name: SharedString::from("Edit"),
                items: vec![
                    MenuItem::action("Undo", Quit),
                    MenuItem::action("Redo", Quit),
                    MenuItem::separator(),
                    MenuItem::action("Cut", Quit),
                    MenuItem::action("Copy", Quit),
                    MenuItem::action("Paste", Quit),
                ],
            },
            Menu {
                name: SharedString::from("View"),
                items: vec![
                    MenuItem::action("Command Palette", ToggleCommandPalette),
                    MenuItem::separator(),
                    MenuItem::action("Toggle File Tree", ToggleFileTree),
                    MenuItem::action("Toggle Bottom Panel", ToggleBottomPanel),
                    MenuItem::separator(),
                    MenuItem::action("Split Right", SplitRight),
                    MenuItem::action("Split Down", SplitDown),
                ],
            },
        ]);
    }

    /// Open the main application window
    fn open_main_window(cx: &mut App) {
        let bounds = WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::Point { x: px(100.0), y: px(100.0) },
            size: gpui::Size {
                width: px(1400.0),
                height: px(900.0),
            },
        });

        let options = WindowOptions {
            window_bounds: Some(bounds),
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("RustIDE")),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: true,
            is_minimizable: true,
            display_id: None,
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            app_id: Some(String::from("com.rustide.app")),
            window_min_size: Some(gpui::Size {
                width: px(800.0),
                height: px(600.0),
            }),
            window_decorations: None,
            tabbing_identifier: None,
        };

        cx.open_window(options, |window, cx| cx.new(|cx| Workspace::new(window, cx)))
            .expect("Failed to open main window");
    }

    /// Get the application state
    pub fn state(&self) -> Arc<RwLock<AppState>> {
        self.state.clone()
    }

    /// Set the workspace root path
    pub fn set_workspace_root(&self, path: PathBuf) {
        self.state.write().workspace_root = Some(path);
    }
}

impl Default for RustideApp {
    fn default() -> Self {
        Self::new()
    }
}
