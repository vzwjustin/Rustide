//! Project file browser panel
//!
//! Displays the workspace file system as a tree structure with
//! expand/collapse functionality.

use gpui::{
    div, prelude::*, px, App, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, Window,
};
use std::cmp::Ordering;
use std::path::PathBuf;

use crate::theme::current_theme;

/// A node in the file tree
#[derive(Clone, Debug)]
pub struct FileNode {
    /// Full path to the file or directory
    pub path: PathBuf,
    /// Display name
    pub name: String,
    /// Whether this is a directory
    pub is_directory: bool,
    /// Whether the directory is expanded (only applies to directories)
    pub is_expanded: bool,
    /// Child nodes (only for directories)
    pub children: Vec<FileNode>,
    /// Depth in the tree (for indentation)
    pub depth: usize,
}

impl FileNode {
    /// Create a new file node
    pub fn new(path: PathBuf, depth: usize) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let is_directory = path.is_dir();

        Self {
            path,
            name,
            is_directory,
            is_expanded: false,
            children: Vec::new(),
            depth,
        }
    }

    /// Load children for a directory
    pub fn load_children(&mut self) {
        if !self.is_directory || !self.children.is_empty() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&self.path) {
            let mut children: Vec<FileNode> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    // Filter out hidden files and common ignored directories
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    !name.starts_with('.')
                        && name != "target"
                        && name != "node_modules"
                        && name != "__pycache__"
                })
                .map(|e| FileNode::new(e.path(), self.depth + 1))
                .collect();

            // Sort: directories first, then alphabetically
            children.sort_by(|a, b| {
                match (a.is_directory, b.is_directory) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                }
            });

            self.children = children;
        }
    }
}

/// File tree component
pub struct FileTree {
    focus_handle: FocusHandle,
    root: Option<FileNode>,
    selected_path: Option<PathBuf>,
}

impl FileTree {
    /// Create a new file tree
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            root: None,
            selected_path: None,
        }
    }

    /// Set the root directory
    pub fn set_root(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let mut root = FileNode::new(path, 0);
        root.is_expanded = true;
        root.load_children();
        self.root = Some(root);
        cx.notify();
    }

    /// Toggle expansion of a directory
    pub fn toggle_expanded(&mut self, path: &PathBuf, cx: &mut Context<Self>) {
        if let Some(ref mut root) = self.root {
            Self::toggle_node_expanded(root, path);
            cx.notify();
        }
    }

    /// Recursively find and toggle a node
    fn toggle_node_expanded(node: &mut FileNode, path: &PathBuf) -> bool {
        if &node.path == path {
            if node.is_directory {
                node.is_expanded = !node.is_expanded;
                if node.is_expanded && node.children.is_empty() {
                    node.load_children();
                }
            }
            return true;
        }

        for child in &mut node.children {
            if Self::toggle_node_expanded(child, path) {
                return true;
            }
        }
        false
    }

    /// Select a file or directory
    pub fn select(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.selected_path = Some(path);
        cx.notify();
    }

    /// Get the selected path
    pub fn selected_path(&self) -> Option<&PathBuf> {
        self.selected_path.as_ref()
    }

    /// Render a single tree node
    fn render_node(&self, node: &FileNode, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();
        let is_selected = self.selected_path.as_ref() == Some(&node.path);

        let icon = if node.is_directory {
            if node.is_expanded {
                "v"
            } else {
                ">"
            }
        } else {
            " "
        };

        let file_icon = if node.is_directory {
            "[]"
        } else {
            "##"
        };

        div()
            .w_full()
            .pl(px((node.depth as f32) * 16.0 + 8.0))
            .pr(px(8.0))
            .py(px(2.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .cursor_pointer()
            .when(is_selected, |d| d.bg(theme.background.selection))
            .when(!is_selected, |d| d.hover(|d| d.bg(theme.background.hover)))
            .child(
                div()
                    .w(px(12.0))
                    .text_color(theme.text.secondary)
                    .text_size(px(10.0))
                    .child(icon)
            )
            .child(
                div()
                    .text_color(if node.is_directory {
                        theme.syntax.type_name
                    } else {
                        theme.text.secondary
                    })
                    .text_size(px(11.0))
                    .child(file_icon)
            )
            .child(
                div()
                    .text_color(theme.text.primary)
                    .child(SharedString::from(node.name.clone()))
            )
    }

    /// Render tree nodes recursively
    fn render_tree(&self, node: &FileNode, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let mut elements = vec![self.render_node(node, cx).into_any_element()];

        if node.is_expanded {
            for child in &node.children {
                elements.extend(self.render_tree(child, cx));
            }
        }

        elements
    }

    /// Render the header
    fn render_header(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .h(px(32.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(12.0))
            .bg(theme.background.sidebar)
            .border_b_1()
            .border_color(theme.border.default)
            .text_color(theme.text.secondary)
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .child("EXPLORER")
    }
}

impl Focusable for FileTree {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileTree {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        let tree_content = div()
            .id("file-tree-content")
            .flex_1()
            .overflow_y_scroll()
            .children(
                self.root
                    .as_ref()
                    .map(|root| self.render_tree(root, cx))
                    .unwrap_or_default()
            );

        div()
            .id("file-tree")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background.sidebar)
            .child(self.render_header(cx))
            .child(tree_content)
    }
}
