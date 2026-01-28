//! Tab-based editor pane container
//!
//! Manages multiple open files in a tabbed interface.

use gpui::{
    div, prelude::*, px, App, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, Window,
};
use std::path::PathBuf;

use crate::theme::current_theme;
use editor_core::Document;

/// Represents an open editor tab
#[derive(Clone)]
pub struct EditorTab {
    /// File path
    pub path: PathBuf,
    /// Display name (filename)
    pub name: String,
    /// Whether the file has unsaved changes
    pub is_modified: bool,
    /// Document entity (GPUI reactive)
    pub document: Entity<Document>,
    /// Whether the document is currently loading
    pub is_loading: bool,
    /// Cursor position (line, column)
    pub cursor: (usize, usize),
}

impl EditorTab {
    /// Create a new editor tab with a document entity
    pub fn new(path: PathBuf, document: Entity<Document>) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        Self {
            path,
            name,
            is_modified: false,
            document,
            is_loading: false,
            cursor: (1, 1),
        }
    }
}

/// Editor pane component
pub struct EditorPane {
    focus_handle: FocusHandle,
    tabs: Vec<EditorTab>,
    active_tab: Option<usize>,
}

impl EditorPane {
    /// Create a new editor pane
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tabs: Vec::new(),
            active_tab: None,
        }
    }

    /// Open a file in a new tab
    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(idx);
            cx.notify();
            return;
        }

        // Create document entity with synchronous load
        // Note: Async loading will be added in plan 01-03
        let document = cx.new(|_| {
            match Document::open(&path) {
                Ok(doc) => doc,
                Err(e) => {
                    tracing::error!("Failed to open file {:?}: {}", path, e);
                    Document::new() // Empty document on error
                }
            }
        });

        let tab = EditorTab::new(path.clone(), document);
        self.tabs.push(tab);
        self.active_tab = Some(self.tabs.len() - 1);
        cx.notify();
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            self.tabs.remove(index);

            // Update active tab
            self.active_tab = if self.tabs.is_empty() {
                None
            } else if let Some(active) = self.active_tab {
                if active >= self.tabs.len() {
                    Some(self.tabs.len() - 1)
                } else if active > index {
                    Some(active - 1)
                } else {
                    Some(active)
                }
            } else {
                None
            };

            cx.notify();
        }
    }

    /// Set the active tab
    pub fn set_active_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            self.active_tab = Some(index);
            cx.notify();
        }
    }

    /// Get the active tab
    pub fn active_tab(&self) -> Option<&EditorTab> {
        self.active_tab.and_then(|idx| self.tabs.get(idx))
    }

    /// Render a single tab
    fn render_tab(&self, tab: &EditorTab, index: usize, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();
        let is_active = self.active_tab == Some(index);

        div()
            .px(px(12.0))
            .py(px(6.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .cursor_pointer()
            .when(is_active, |d| {
                d.bg(theme.background.editor)
                    .text_color(theme.text.primary)
                    .border_b_2()
                    .border_color(theme.accent.primary)
            })
            .when(!is_active, |d| {
                d.text_color(theme.text.secondary)
                    .hover(|d| d.bg(theme.background.hover))
            })
            .child(SharedString::from(tab.name.clone()))
            .when(tab.is_modified, |d| {
                d.child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded_full()
                        .bg(theme.accent.primary)
                )
            })
    }

    /// Render the tab bar
    fn render_tab_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .h(px(36.0))
            .w_full()
            .flex()
            .items_center()
            .bg(theme.background.panel)
            .border_b_1()
            .border_color(theme.border.default)
            .children(
                self.tabs
                    .iter()
                    .enumerate()
                    .map(|(i, tab)| self.render_tab(tab, i, cx))
            )
    }

    /// Render the editor content area
    fn render_editor_content(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        if let Some(tab) = self.active_tab() {
            // Show loading indicator while file is being loaded asynchronously
            if tab.is_loading {
                return div()
                    .id("loading")
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.background.editor)
                    .text_color(theme.text.secondary)
                    .child("Loading...");
            }

            // Read line count from Document entity
            let line_count = tab.document.read(cx).line_count();
            let document = tab.document.clone();

            div()
                .id("editor-content")
                .flex_1()
                .overflow_y_scroll()
                .bg(theme.background.editor)
                .p(px(8.0))
                .font_family(theme.fonts.mono_family.clone())
                .text_size(px(theme.fonts.mono_size))
                .children((0..line_count).map(|i| {
                    // Read each line from Document
                    let line_text = document
                        .read(cx)
                        .line(i)
                        .unwrap_or_default();

                    div()
                        .flex()
                        .child(
                            div()
                                .w(px(40.0))
                                .text_right()
                                .pr(px(12.0))
                                .text_color(theme.text.line_number)
                                .child(SharedString::from(format!("{}", i + 1)))
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_color(theme.text.primary)
                                .child(if line_text.is_empty() {
                                    SharedString::from(" ")
                                } else {
                                    SharedString::from(line_text)
                                })
                        )
                }))
        } else {
            div()
                .id("empty-editor")
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.background.editor)
                .text_color(theme.text.secondary)
                .child("No file open - Press Cmd+O to open a file")
        }
    }

    /// Render the status bar
    fn render_status_bar(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        let (line, col) = self
            .active_tab()
            .map(|t| t.cursor)
            .unwrap_or((1, 1));

        let file_info = self
            .active_tab()
            .map(|t| t.path.to_string_lossy().to_string())
            .unwrap_or_else(|| "No file".to_string());

        div()
            .h(px(24.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(12.0))
            .bg(theme.background.statusbar)
            .text_color(theme.text.secondary)
            .text_size(px(11.0))
            .child(SharedString::from(file_info))
            .child(SharedString::from(format!("Ln {}, Col {}", line, col)))
    }
}

impl Focusable for EditorPane {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("editor-pane")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background.editor)
            .child(self.render_tab_bar(cx))
            .child(self.render_editor_content(cx))
            .child(self.render_status_bar(cx))
    }
}
