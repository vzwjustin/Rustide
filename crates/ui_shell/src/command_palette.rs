//! Fuzzy search command palette
//!
//! Provides a searchable list of all available commands.

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use gpui::{
    div, prelude::*, px, App, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement,
    SharedString, Styled, Window,
};

use crate::theme::current_theme;

/// A command that can be executed from the palette
#[derive(Clone, Debug)]
pub struct PaletteCommand {
    /// Unique identifier for the command
    pub id: String,
    /// Display name
    pub name: String,
    /// Keyboard shortcut (if any)
    pub shortcut: Option<String>,
    /// Category for grouping
    pub category: Option<String>,
}

impl PaletteCommand {
    /// Create a new palette command
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            shortcut: None,
            category: None,
        }
    }

    /// Set the keyboard shortcut
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set the category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Filtered command with match score
struct FilteredCommand {
    command: PaletteCommand,
    score: i64,
}

/// Command palette component
pub struct CommandPalette {
    focus_handle: FocusHandle,
    query: String,
    commands: Vec<PaletteCommand>,
    filtered_commands: Vec<FilteredCommand>,
    selected_index: usize,
    matcher: SkimMatcherV2,
}

impl CommandPalette {
    /// Create a new command palette
    pub fn new(cx: &mut Context<Self>) -> Self {
        let commands = Self::default_commands();
        let filtered_commands: Vec<FilteredCommand> = commands
            .iter()
            .map(|c| FilteredCommand {
                command: c.clone(),
                score: 0,
            })
            .collect();

        Self {
            focus_handle: cx.focus_handle(),
            query: String::new(),
            commands,
            filtered_commands,
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Get default commands
    fn default_commands() -> Vec<PaletteCommand> {
        vec![
            PaletteCommand::new("file.new", "New File")
                .with_shortcut("Cmd+N")
                .with_category("File"),
            PaletteCommand::new("file.open", "Open File...")
                .with_shortcut("Cmd+O")
                .with_category("File"),
            PaletteCommand::new("file.save", "Save")
                .with_shortcut("Cmd+S")
                .with_category("File"),
            PaletteCommand::new("file.save_as", "Save As...")
                .with_shortcut("Cmd+Shift+S")
                .with_category("File"),
            PaletteCommand::new("view.file_tree", "Toggle File Tree")
                .with_shortcut("Cmd+B")
                .with_category("View"),
            PaletteCommand::new("view.bottom_panel", "Toggle Bottom Panel")
                .with_shortcut("Cmd+J")
                .with_category("View"),
            PaletteCommand::new("view.command_palette", "Command Palette")
                .with_shortcut("Cmd+Shift+P")
                .with_category("View"),
        ]
    }

    /// Update the search query
    pub fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query.clone();
        self.filter_commands(&query);
        self.selected_index = 0;
        cx.notify();
    }

    /// Filter commands based on query
    fn filter_commands(&mut self, query: &str) {
        if query.is_empty() {
            self.filtered_commands = self
                .commands
                .iter()
                .map(|c| FilteredCommand {
                    command: c.clone(),
                    score: 0,
                })
                .collect();
        } else {
            let mut filtered: Vec<FilteredCommand> = self
                .commands
                .iter()
                .filter_map(|cmd| {
                    self.matcher
                        .fuzzy_match(&cmd.name, query)
                        .map(|score| FilteredCommand {
                            command: cmd.clone(),
                            score,
                        })
                })
                .collect();
            filtered.sort_by(|a, b| b.score.cmp(&a.score));
            self.filtered_commands = filtered;
        }
    }

    /// Select the next item
    pub fn select_next(&mut self, cx: &mut Context<Self>) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_commands.len();
            cx.notify();
        }
    }

    /// Select the previous item
    pub fn select_prev(&mut self, cx: &mut Context<Self>) {
        if !self.filtered_commands.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.filtered_commands.len() - 1
            } else {
                self.selected_index - 1
            };
            cx.notify();
        }
    }

    /// Execute the selected command
    pub fn execute_selected(&mut self, _cx: &mut Context<Self>) -> Option<String> {
        self.filtered_commands
            .get(self.selected_index)
            .map(|fc| fc.command.id.clone())
    }

    /// Render the search input
    fn render_input(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .w_full()
            .p(px(12.0))
            .border_b_1()
            .border_color(theme.border.default)
            .child(
                div()
                    .w_full()
                    .px(px(8.0))
                    .py(px(6.0))
                    .bg(theme.background.editor)
                    .rounded(px(4.0))
                    .text_color(theme.text.primary)
                    .child(if self.query.is_empty() {
                        SharedString::from("Type to search commands...")
                    } else {
                        SharedString::from(self.query.clone())
                    })
            )
    }

    /// Render a single command item
    fn render_command_item(
        &self,
        command: &PaletteCommand,
        index: usize,
        _score: i64,
        _cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = current_theme();
        let is_selected = index == self.selected_index;

        div()
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .flex()
            .justify_between()
            .items_center()
            .cursor_pointer()
            .when(is_selected, |d| d.bg(theme.background.selection))
            .when(!is_selected, |d| d.hover(|d| d.bg(theme.background.hover)))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_color(theme.text.primary)
                            .child(SharedString::from(command.name.clone()))
                    )
                    .when_some(command.category.as_ref(), |d, cat| {
                        d.child(
                            div()
                                .text_color(theme.text.secondary)
                                .text_size(px(11.0))
                                .child(SharedString::from(cat.clone()))
                        )
                    })
            )
            .when_some(command.shortcut.as_ref(), |d, shortcut| {
                d.child(
                    div()
                        .text_color(theme.text.secondary)
                        .text_size(px(11.0))
                        .child(SharedString::from(shortcut.clone()))
                )
            })
    }

    /// Render the command list
    fn render_list(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("command-list")
            .flex_1()
            .overflow_y_scroll()
            .children(
                self.filtered_commands
                    .iter()
                    .enumerate()
                    .map(|(i, fc)| self.render_command_item(&fc.command, i, fc.score, cx))
            )
            .when(self.filtered_commands.is_empty(), |d| {
                d.child(
                    div()
                        .p(px(16.0))
                        .text_color(theme.text.secondary)
                        .child("No matching commands")
                )
            })
    }
}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("command-palette")
            .track_focus(&self.focus_handle)
            .w(px(500.0))
            .max_h(px(400.0))
            .bg(theme.background.panel)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(8.0))
            .shadow_lg()
            .flex()
            .flex_col()
            .child(self.render_input(cx))
            .child(self.render_list(cx))
    }
}
