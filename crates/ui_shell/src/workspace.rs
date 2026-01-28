//! Workspace view with panes/docks layout
//!
//! The Workspace is the main container view that orchestrates
//! the file tree, editor panes, and bottom panel.

use gpui::{
    div, prelude::*, px, App, Entity, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement,
    Styled, Window,
};
use std::path::PathBuf;

use crate::bottom_panel::BottomPanel;
use crate::command_palette::CommandPalette;
use crate::editor_pane::EditorPane;
use crate::file_tree::FileTree;
use crate::theme::current_theme;

/// Workspace events
pub enum WorkspaceEvent {
    FileOpened(PathBuf),
    FileClosed(PathBuf),
    FocusChanged,
}

impl EventEmitter<WorkspaceEvent> for Workspace {}

/// The main workspace view
pub struct Workspace {
    /// Focus handle for keyboard navigation
    focus_handle: FocusHandle,
    /// File tree panel (left sidebar)
    file_tree: Entity<FileTree>,
    /// Main editor pane
    editor_pane: Entity<EditorPane>,
    /// Bottom panel (terminal, problems, output)
    bottom_panel: Entity<BottomPanel>,
    /// Command palette overlay
    command_palette: Option<Entity<CommandPalette>>,
    /// Whether file tree is visible
    file_tree_visible: bool,
    /// Whether bottom panel is visible
    bottom_panel_visible: bool,
    /// Width of the file tree in pixels
    file_tree_width: f32,
    /// Height of the bottom panel in pixels
    bottom_panel_height: f32,
    /// Current workspace root
    workspace_root: Option<PathBuf>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let file_tree = cx.new(|cx| FileTree::new(cx));
        let editor_pane = cx.new(|cx| EditorPane::new(cx));
        let bottom_panel = cx.new(|cx| BottomPanel::new(cx));

        // Try to use current directory as workspace root
        let workspace_root = std::env::current_dir().ok();

        // Update file tree with workspace root
        if let Some(ref root) = workspace_root {
            let root_clone = root.clone();
            file_tree.update(cx, |tree, cx| {
                tree.set_root(root_clone, cx);
            });
        }

        Self {
            focus_handle: cx.focus_handle(),
            file_tree,
            editor_pane,
            bottom_panel,
            command_palette: None,
            file_tree_visible: true,
            bottom_panel_visible: true,
            file_tree_width: 250.0,
            bottom_panel_height: 200.0,
            workspace_root,
        }
    }

    /// Toggle the file tree visibility
    pub fn toggle_file_tree(&mut self, cx: &mut Context<Self>) {
        self.file_tree_visible = !self.file_tree_visible;
        cx.notify();
    }

    /// Toggle the bottom panel visibility
    pub fn toggle_bottom_panel(&mut self, cx: &mut Context<Self>) {
        self.bottom_panel_visible = !self.bottom_panel_visible;
        cx.notify();
    }

    /// Toggle the command palette
    pub fn toggle_command_palette(&mut self, cx: &mut Context<Self>) {
        if self.command_palette.is_some() {
            self.command_palette = None;
        } else {
            self.command_palette = Some(cx.new(|cx| CommandPalette::new(cx)));
        }
        cx.notify();
    }

    /// Open a file in the editor
    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let path_clone = path.clone();
        self.editor_pane.update(cx, |pane, cx| {
            pane.open_file(path_clone, cx);
        });
        cx.emit(WorkspaceEvent::FileOpened(path));
    }

    /// Set the workspace root directory
    pub fn set_workspace_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.workspace_root = Some(path.clone());
        self.file_tree.update(cx, |tree, cx| {
            tree.set_root(path, cx);
        });
        cx.notify();
    }

    /// Get the current workspace root
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.workspace_root.as_ref()
    }

    /// Render the file tree panel
    fn render_file_tree(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .w(px(self.file_tree_width))
            .h_full()
            .bg(theme.background.sidebar)
            .border_r_1()
            .border_color(theme.border.default)
            .flex_shrink_0()
            .child(self.file_tree.clone())
    }

    /// Render the main editor area
    fn render_editor_area(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .flex_1()
            .h_full()
            .bg(theme.background.editor)
            .child(self.editor_pane.clone())
    }

    /// Render the bottom panel
    fn render_bottom_panel(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .w_full()
            .h(px(self.bottom_panel_height))
            .bg(theme.background.panel)
            .border_t_1()
            .border_color(theme.border.default)
            .flex_shrink_0()
            .child(self.bottom_panel.clone())
    }

    /// Render the command palette overlay
    fn render_command_palette(&self, _cx: &Context<Self>) -> Option<impl IntoElement> {
        let theme = current_theme();

        self.command_palette.as_ref().map(|palette| {
            div()
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .pt(px(80.0))
                .bg(theme.background.overlay)
                .child(palette.clone())
        })
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        let mut workspace = div()
            .id("workspace")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(theme.background.base)
            .text_color(theme.text.primary)
            .font_family(theme.fonts.ui_family.clone())
            .text_size(px(theme.fonts.ui_size))
            .flex()
            .flex_col();

        // Main content area (horizontal layout)
        let mut main_content = div()
            .flex_1()
            .flex()
            .flex_row()
            .overflow_hidden();

        // Add file tree if visible
        if self.file_tree_visible {
            main_content = main_content.child(self.render_file_tree(cx));
        }

        // Add editor area
        main_content = main_content.child(self.render_editor_area(cx));

        workspace = workspace.child(main_content);

        // Add bottom panel if visible
        if self.bottom_panel_visible {
            workspace = workspace.child(self.render_bottom_panel(cx));
        }

        // Add command palette overlay if open
        if let Some(palette) = &self.command_palette {
            workspace = workspace.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .justify_center()
                    .pt(px(80.0))
                    .bg(theme.background.overlay)
                    .child(palette.clone())
            );
        }

        workspace
    }
}

// Action handlers
impl Workspace {
    pub fn handle_toggle_file_tree(
        &mut self,
        _: &crate::app::ToggleFileTree,
        cx: &mut Context<Self>,
    ) {
        self.toggle_file_tree(cx);
    }

    pub fn handle_toggle_bottom_panel(
        &mut self,
        _: &crate::app::ToggleBottomPanel,
        cx: &mut Context<Self>,
    ) {
        self.toggle_bottom_panel(cx);
    }

    pub fn handle_toggle_command_palette(
        &mut self,
        _: &crate::app::ToggleCommandPalette,
        cx: &mut Context<Self>,
    ) {
        self.toggle_command_palette(cx);
    }
}
