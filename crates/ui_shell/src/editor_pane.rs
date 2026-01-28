//! Tab-based editor pane container
//!
//! Manages multiple open files in a tabbed interface.

use gpui::{
    div, prelude::*, px, uniform_list, App, AsyncApp, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, UniformListScrollHandle, WeakEntity, Window,
};
use std::path::PathBuf;

use crate::theme::current_theme;
use editor_core::Document;

/// Uniform line height for virtual scrolling (required by uniform_list)
const LINE_HEIGHT: f32 = 20.0;

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
    /// Scroll handle for virtual scrolling (tracks scroll position)
    scroll_handle: UniformListScrollHandle,
}

impl EditorPane {
    /// Create a new editor pane
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            tabs: Vec::new(),
            active_tab: None,
            scroll_handle: UniformListScrollHandle::new(),
        }
    }

    /// Open a file in a new tab (async loading)
    ///
    /// Uses cx.spawn and background_executor to load files without blocking the UI.
    /// The tab is created immediately in a loading state, then populated when
    /// the background file read completes.
    pub fn open_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Check if file is already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path == path) {
            self.active_tab = Some(idx);
            cx.notify();
            return;
        }

        // Create empty document entity (placeholder during loading)
        let document = cx.new(|_| Document::new());

        // Create tab in loading state
        let tab = EditorTab {
            document: document.clone(),
            path: path.clone(),
            name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "untitled".to_string()),
            is_modified: false,
            cursor: (1, 1),
            is_loading: true, // Start in loading state
        };

        self.tabs.push(tab);
        let tab_index = self.tabs.len() - 1;
        self.active_tab = Some(tab_index);
        cx.notify(); // Show loading state immediately

        // Spawn async loading task using GPUI's async executor
        cx.spawn(async move |this: WeakEntity<EditorPane>, cx: &mut AsyncApp| {
            // Read file on background thread (non-blocking)
            let result: Result<String, std::io::Error> = cx
                .background_executor()
                .spawn(async move { tokio::fs::read_to_string(&path).await })
                .await;

            // Update on main thread
            let _ = this.update(cx, |pane, cx| {
                match result {
                    Ok(content) => {
                        // Update document with loaded content
                        document.update(cx, |doc, _| {
                            *doc = Document::from_text(&content);
                        });

                        // Mark loading complete
                        if let Some(tab) = pane.tabs.get_mut(tab_index) {
                            tab.is_loading = false;
                        }
                        tracing::info!(
                            "File loaded: {} lines",
                            document.read(cx).line_count()
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to load file: {}", e);
                        // Mark loading complete even on error (shows empty document)
                        if let Some(tab) = pane.tabs.get_mut(tab_index) {
                            tab.is_loading = false;
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
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

    /// Render a single line element for uniform_list
    fn render_line(line_idx: usize, line_text: &str, theme: &crate::theme::Theme) -> impl IntoElement {
        div()
            .h(px(LINE_HEIGHT))
            .flex()
            .items_center()
            .child(
                div()
                    .w(px(48.0))
                    .text_right()
                    .pr(px(12.0))
                    .text_color(theme.text.line_number)
                    .child(SharedString::from(format!("{}", line_idx + 1)))
            )
            .child(
                div()
                    .flex_1()
                    .text_color(theme.text.primary)
                    .child(if line_text.is_empty() {
                        SharedString::from(" ")
                    } else {
                        SharedString::from(line_text.to_string())
                    })
            )
    }

    /// Render the editor content area with virtual scrolling
    ///
    /// Uses uniform_list for O(visible) rendering complexity instead of O(total).
    /// Only visible lines + buffer are rendered, enabling smooth scrolling of 100K+ line files.
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
            let scroll_handle = self.scroll_handle.clone();

            // Use uniform_list for virtual scrolling - only renders visible lines
            div()
                .id("editor-content")
                .flex_1()
                .overflow_hidden() // Required for uniform_list to work properly
                .bg(theme.background.editor)
                .font_family(theme.fonts.mono_family.clone())
                .text_size(px(theme.fonts.mono_size))
                .child(
                    uniform_list(
                        "editor-lines",
                        line_count,
                        move |visible_range, _window, cx| {
                            let doc = document.read(cx);
                            let theme = current_theme();
                            visible_range
                                .map(|line_idx| {
                                    let line_text = doc.line(line_idx).unwrap_or_default();
                                    Self::render_line(line_idx, &line_text, &theme)
                                })
                                .collect()
                        },
                    )
                    .track_scroll(scroll_handle)
                    .flex_1()
                )
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
