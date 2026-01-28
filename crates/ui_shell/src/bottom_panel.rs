//! Terminal/output/problems panel
//!
//! The bottom panel provides tabs for terminal, problems, and output.

use gpui::{
    div, prelude::*, px, App, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, Window,
};

use crate::theme::current_theme;

/// Bottom panel tab type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BottomPanelTab {
    Terminal,
    Problems,
    Output,
    DebugConsole,
}

/// A line in the terminal output
#[derive(Clone, Debug)]
pub struct TerminalLine {
    pub content: String,
    pub is_error: bool,
    pub timestamp: Option<String>,
}

/// A problem/diagnostic item
#[derive(Clone, Debug)]
pub struct Problem {
    pub severity: ProblemSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Severity of a problem
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Bottom panel component
pub struct BottomPanel {
    focus_handle: FocusHandle,
    active_tab: BottomPanelTab,
    terminal_lines: Vec<TerminalLine>,
    problems: Vec<Problem>,
    output_lines: Vec<String>,
    is_maximized: bool,
}

impl BottomPanel {
    /// Create a new bottom panel
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            active_tab: BottomPanelTab::Terminal,
            terminal_lines: Vec::new(),
            problems: Vec::new(),
            output_lines: Vec::new(),
            is_maximized: false,
        }
    }

    /// Set the active tab
    pub fn set_active_tab(&mut self, tab: BottomPanelTab, cx: &mut Context<Self>) {
        self.active_tab = tab;
        cx.notify();
    }

    /// Add a terminal line
    pub fn add_terminal_line(&mut self, line: TerminalLine, cx: &mut Context<Self>) {
        self.terminal_lines.push(line);
        cx.notify();
    }

    /// Add a problem
    pub fn add_problem(&mut self, problem: Problem, cx: &mut Context<Self>) {
        self.problems.push(problem);
        cx.notify();
    }

    /// Clear all problems
    pub fn clear_problems(&mut self, cx: &mut Context<Self>) {
        self.problems.clear();
        cx.notify();
    }

    /// Add an output line
    pub fn add_output(&mut self, line: String, cx: &mut Context<Self>) {
        self.output_lines.push(line);
        cx.notify();
    }

    /// Render the tab bar
    fn render_tab_bar(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();
        let tabs = [
            (BottomPanelTab::Terminal, "Terminal"),
            (BottomPanelTab::Problems, "Problems"),
            (BottomPanelTab::Output, "Output"),
            (BottomPanelTab::DebugConsole, "Debug Console"),
        ];

        div()
            .h(px(32.0))
            .w_full()
            .flex()
            .items_center()
            .px(px(8.0))
            .bg(theme.background.panel)
            .border_b_1()
            .border_color(theme.border.default)
            .children(tabs.iter().map(|(tab, label)| {
                let is_active = self.active_tab == *tab;
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .cursor_pointer()
                    .when(is_active, |d| {
                        d.bg(theme.background.editor)
                            .text_color(theme.text.primary)
                    })
                    .when(!is_active, |d| {
                        d.text_color(theme.text.secondary)
                            .hover(|d| d.bg(theme.background.hover))
                    })
                    .child(SharedString::from(*label))
            }))
    }

    /// Render terminal content
    fn render_terminal(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("terminal-content")
            .flex_1()
            .p(px(8.0))
            .overflow_y_scroll()
            .font_family(theme.fonts.mono_family.clone())
            .text_size(px(theme.fonts.mono_size))
            .children(self.terminal_lines.iter().map(|line| {
                let color = if line.is_error {
                    theme.diagnostic.error
                } else {
                    theme.text.primary
                };
                div()
                    .text_color(color)
                    .child(SharedString::from(line.content.clone()))
            }))
    }

    /// Render problems content
    fn render_problems(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("problems-content")
            .flex_1()
            .p(px(8.0))
            .overflow_y_scroll()
            .children(self.problems.iter().map(|problem| {
                let severity_color = match problem.severity {
                    ProblemSeverity::Error => theme.diagnostic.error,
                    ProblemSeverity::Warning => theme.diagnostic.warning,
                    ProblemSeverity::Info => theme.diagnostic.info,
                    ProblemSeverity::Hint => theme.diagnostic.hint,
                };
                let location = match (&problem.file, problem.line) {
                    (Some(file), Some(line)) => format!("{}:{}", file, line),
                    (Some(file), None) => file.clone(),
                    _ => String::new(),
                };

                div()
                    .flex()
                    .gap(px(8.0))
                    .py(px(2.0))
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded_full()
                            .bg(severity_color)
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme.text.primary)
                            .child(SharedString::from(problem.message.clone()))
                    )
                    .child(
                        div()
                            .text_color(theme.text.secondary)
                            .child(SharedString::from(location))
                    )
            }))
    }

    /// Render output content
    fn render_output(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("output-content")
            .flex_1()
            .p(px(8.0))
            .overflow_y_scroll()
            .font_family(theme.fonts.mono_family.clone())
            .text_size(px(theme.fonts.mono_size))
            .children(self.output_lines.iter().map(|line| {
                div()
                    .text_color(theme.text.primary)
                    .child(SharedString::from(line.clone()))
            }))
    }

    /// Render debug console content
    fn render_debug_console(&self, _cx: &Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("debug-console-content")
            .flex_1()
            .p(px(8.0))
            .overflow_y_scroll()
            .flex()
            .items_center()
            .justify_center()
            .text_color(theme.text.secondary)
            .child("Debug console - not connected")
    }

    /// Render the content area based on active tab
    fn render_content(&self, cx: &Context<Self>) -> gpui::AnyElement {
        match self.active_tab {
            BottomPanelTab::Terminal => self.render_terminal(cx).into_any_element(),
            BottomPanelTab::Problems => self.render_problems(cx).into_any_element(),
            BottomPanelTab::Output => self.render_output(cx).into_any_element(),
            BottomPanelTab::DebugConsole => self.render_debug_console(cx).into_any_element(),
        }
    }
}

impl Focusable for BottomPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BottomPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = current_theme();

        div()
            .id("bottom-panel")
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background.panel)
            .child(self.render_tab_bar(cx))
            .child(self.render_content(cx))
    }
}
