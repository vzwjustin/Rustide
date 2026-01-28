//! Theme and styling definitions for RustIDE
//!
//! Provides a comprehensive dark theme with colors, fonts, and spacing.

use gpui::{hsla, Hsla, SharedString};

/// Main theme struct containing all styling definitions
#[derive(Clone, Debug)]
pub struct Theme {
    /// Background colors
    pub background: BackgroundColors,
    /// Text colors
    pub text: TextColors,
    /// Border colors
    pub border: BorderColors,
    /// Syntax highlighting colors
    pub syntax: SyntaxColors,
    /// Diagnostic colors
    pub diagnostic: DiagnosticColors,
    /// Accent colors
    pub accent: AccentColors,
    /// UI element colors
    pub ui: UiColors,
    /// Font settings
    pub fonts: FontSettings,
    /// Spacing and sizing
    pub spacing: Spacing,
}

#[derive(Clone, Debug)]
pub struct BackgroundColors {
    pub base: Hsla,
    pub surface: Hsla,
    pub elevated: Hsla,
    pub overlay: Hsla,
    pub sidebar: Hsla,
    pub editor: Hsla,
    pub panel: Hsla,
    pub selected: Hsla,
    pub selection: Hsla,
    pub hover: Hsla,
    pub statusbar: Hsla,
}

#[derive(Clone, Debug)]
pub struct TextColors {
    pub primary: Hsla,
    pub secondary: Hsla,
    pub muted: Hsla,
    pub disabled: Hsla,
    pub accent: Hsla,
    pub link: Hsla,
    pub error: Hsla,
    pub warning: Hsla,
    pub success: Hsla,
    pub line_number: Hsla,
}

#[derive(Clone, Debug)]
pub struct BorderColors {
    pub default: Hsla,
    pub focused: Hsla,
    pub muted: Hsla,
    pub transparent: Hsla,
}

#[derive(Clone, Debug)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub function: Hsla,
    pub variable: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub type_name: Hsla,
    pub constant: Hsla,
    pub operator: Hsla,
    pub punctuation: Hsla,
    pub attribute: Hsla,
    pub tag: Hsla,
}

#[derive(Clone, Debug)]
pub struct DiagnosticColors {
    pub error: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    pub hint: Hsla,
}

#[derive(Clone, Debug)]
pub struct AccentColors {
    pub primary: Hsla,
    pub secondary: Hsla,
}

#[derive(Clone, Debug)]
pub struct UiColors {
    pub button_primary: Hsla,
    pub button_secondary: Hsla,
    pub input_background: Hsla,
    pub scrollbar: Hsla,
    pub scrollbar_hover: Hsla,
    pub tab_active: Hsla,
    pub tab_inactive: Hsla,
    pub icon: Hsla,
    pub icon_muted: Hsla,
    pub cursor: Hsla,
}

#[derive(Clone, Debug)]
pub struct FontSettings {
    pub ui_family: SharedString,
    pub mono_family: SharedString,
    pub ui_size: f32,
    pub mono_size: f32,
    pub editor_size: f32,
    pub line_height: f32,
}

#[derive(Clone, Debug)]
pub struct Spacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub panel_padding: f32,
    pub item_gap: f32,
    pub border_radius: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    /// Create a dark theme (default)
    pub fn dark() -> Self {
        Self {
            background: BackgroundColors {
                base: hsla(220.0 / 360.0, 0.13, 0.10, 1.0),
                surface: hsla(220.0 / 360.0, 0.13, 0.12, 1.0),
                elevated: hsla(220.0 / 360.0, 0.13, 0.14, 1.0),
                overlay: hsla(220.0 / 360.0, 0.13, 0.08, 0.95),
                sidebar: hsla(220.0 / 360.0, 0.13, 0.11, 1.0),
                editor: hsla(220.0 / 360.0, 0.13, 0.10, 1.0),
                panel: hsla(220.0 / 360.0, 0.13, 0.09, 1.0),
                selected: hsla(215.0 / 360.0, 0.50, 0.25, 1.0),
                selection: hsla(215.0 / 360.0, 0.50, 0.25, 1.0),
                hover: hsla(220.0 / 360.0, 0.13, 0.16, 1.0),
                statusbar: hsla(220.0 / 360.0, 0.13, 0.08, 1.0),
            },
            text: TextColors {
                primary: hsla(0.0, 0.0, 0.93, 1.0),
                secondary: hsla(0.0, 0.0, 0.73, 1.0),
                muted: hsla(0.0, 0.0, 0.53, 1.0),
                disabled: hsla(0.0, 0.0, 0.40, 1.0),
                accent: hsla(215.0 / 360.0, 0.80, 0.65, 1.0),
                link: hsla(210.0 / 360.0, 0.80, 0.60, 1.0),
                error: hsla(0.0 / 360.0, 0.70, 0.60, 1.0),
                warning: hsla(40.0 / 360.0, 0.70, 0.55, 1.0),
                success: hsla(140.0 / 360.0, 0.60, 0.50, 1.0),
                line_number: hsla(0.0, 0.0, 0.45, 1.0),
            },
            border: BorderColors {
                default: hsla(220.0 / 360.0, 0.13, 0.20, 1.0),
                focused: hsla(215.0 / 360.0, 0.70, 0.55, 1.0),
                muted: hsla(220.0 / 360.0, 0.13, 0.15, 1.0),
                transparent: hsla(0.0, 0.0, 0.0, 0.0),
            },
            syntax: SyntaxColors {
                keyword: hsla(280.0 / 360.0, 0.60, 0.70, 1.0),
                function: hsla(200.0 / 360.0, 0.70, 0.65, 1.0),
                variable: hsla(0.0, 0.0, 0.90, 1.0),
                string: hsla(100.0 / 360.0, 0.50, 0.60, 1.0),
                number: hsla(30.0 / 360.0, 0.80, 0.65, 1.0),
                comment: hsla(0.0, 0.0, 0.50, 1.0),
                type_name: hsla(180.0 / 360.0, 0.60, 0.60, 1.0),
                constant: hsla(30.0 / 360.0, 0.80, 0.65, 1.0),
                operator: hsla(0.0, 0.0, 0.80, 1.0),
                punctuation: hsla(0.0, 0.0, 0.70, 1.0),
                attribute: hsla(50.0 / 360.0, 0.70, 0.65, 1.0),
                tag: hsla(0.0 / 360.0, 0.65, 0.65, 1.0),
            },
            diagnostic: DiagnosticColors {
                error: hsla(0.0 / 360.0, 0.70, 0.60, 1.0),
                warning: hsla(40.0 / 360.0, 0.70, 0.55, 1.0),
                info: hsla(200.0 / 360.0, 0.70, 0.60, 1.0),
                hint: hsla(140.0 / 360.0, 0.50, 0.55, 1.0),
            },
            accent: AccentColors {
                primary: hsla(215.0 / 360.0, 0.70, 0.55, 1.0),
                secondary: hsla(280.0 / 360.0, 0.60, 0.60, 1.0),
            },
            ui: UiColors {
                button_primary: hsla(215.0 / 360.0, 0.70, 0.50, 1.0),
                button_secondary: hsla(220.0 / 360.0, 0.13, 0.20, 1.0),
                input_background: hsla(220.0 / 360.0, 0.13, 0.08, 1.0),
                scrollbar: hsla(0.0, 0.0, 0.30, 0.5),
                scrollbar_hover: hsla(0.0, 0.0, 0.40, 0.7),
                tab_active: hsla(220.0 / 360.0, 0.13, 0.12, 1.0),
                tab_inactive: hsla(220.0 / 360.0, 0.13, 0.10, 1.0),
                icon: hsla(0.0, 0.0, 0.75, 1.0),
                icon_muted: hsla(0.0, 0.0, 0.50, 1.0),
                cursor: hsla(215.0 / 360.0, 0.80, 0.65, 1.0),
            },
            fonts: FontSettings {
                ui_family: SharedString::from("Inter"),
                mono_family: SharedString::from("JetBrains Mono"),
                ui_size: 13.0,
                mono_size: 13.0,
                editor_size: 14.0,
                line_height: 1.5,
            },
            spacing: Spacing {
                xs: 4.0,
                sm: 8.0,
                md: 12.0,
                lg: 16.0,
                xl: 24.0,
                panel_padding: 12.0,
                item_gap: 4.0,
                border_radius: 4.0,
            },
        }
    }

    /// Create a light theme
    pub fn light() -> Self {
        Self {
            background: BackgroundColors {
                base: hsla(0.0, 0.0, 0.98, 1.0),
                surface: hsla(0.0, 0.0, 0.96, 1.0),
                elevated: hsla(0.0, 0.0, 1.0, 1.0),
                overlay: hsla(0.0, 0.0, 1.0, 0.95),
                sidebar: hsla(0.0, 0.0, 0.97, 1.0),
                editor: hsla(0.0, 0.0, 1.0, 1.0),
                panel: hsla(0.0, 0.0, 0.96, 1.0),
                selected: hsla(215.0 / 360.0, 0.60, 0.90, 1.0),
                selection: hsla(215.0 / 360.0, 0.60, 0.90, 1.0),
                hover: hsla(0.0, 0.0, 0.94, 1.0),
                statusbar: hsla(0.0, 0.0, 0.92, 1.0),
            },
            text: TextColors {
                primary: hsla(0.0, 0.0, 0.10, 1.0),
                secondary: hsla(0.0, 0.0, 0.35, 1.0),
                muted: hsla(0.0, 0.0, 0.50, 1.0),
                disabled: hsla(0.0, 0.0, 0.65, 1.0),
                accent: hsla(215.0 / 360.0, 0.80, 0.45, 1.0),
                link: hsla(210.0 / 360.0, 0.80, 0.45, 1.0),
                error: hsla(0.0 / 360.0, 0.70, 0.50, 1.0),
                warning: hsla(40.0 / 360.0, 0.70, 0.45, 1.0),
                success: hsla(140.0 / 360.0, 0.60, 0.40, 1.0),
                line_number: hsla(0.0, 0.0, 0.55, 1.0),
            },
            border: BorderColors {
                default: hsla(0.0, 0.0, 0.85, 1.0),
                focused: hsla(215.0 / 360.0, 0.70, 0.50, 1.0),
                muted: hsla(0.0, 0.0, 0.90, 1.0),
                transparent: hsla(0.0, 0.0, 0.0, 0.0),
            },
            syntax: SyntaxColors {
                keyword: hsla(280.0 / 360.0, 0.70, 0.45, 1.0),
                function: hsla(200.0 / 360.0, 0.80, 0.40, 1.0),
                variable: hsla(0.0, 0.0, 0.15, 1.0),
                string: hsla(100.0 / 360.0, 0.60, 0.40, 1.0),
                number: hsla(30.0 / 360.0, 0.90, 0.45, 1.0),
                comment: hsla(0.0, 0.0, 0.55, 1.0),
                type_name: hsla(180.0 / 360.0, 0.70, 0.40, 1.0),
                constant: hsla(30.0 / 360.0, 0.90, 0.45, 1.0),
                operator: hsla(0.0, 0.0, 0.30, 1.0),
                punctuation: hsla(0.0, 0.0, 0.40, 1.0),
                attribute: hsla(50.0 / 360.0, 0.80, 0.45, 1.0),
                tag: hsla(0.0 / 360.0, 0.75, 0.50, 1.0),
            },
            diagnostic: DiagnosticColors {
                error: hsla(0.0 / 360.0, 0.70, 0.50, 1.0),
                warning: hsla(40.0 / 360.0, 0.70, 0.45, 1.0),
                info: hsla(200.0 / 360.0, 0.70, 0.50, 1.0),
                hint: hsla(140.0 / 360.0, 0.50, 0.45, 1.0),
            },
            accent: AccentColors {
                primary: hsla(215.0 / 360.0, 0.70, 0.50, 1.0),
                secondary: hsla(280.0 / 360.0, 0.60, 0.50, 1.0),
            },
            ui: UiColors {
                button_primary: hsla(215.0 / 360.0, 0.70, 0.50, 1.0),
                button_secondary: hsla(0.0, 0.0, 0.90, 1.0),
                input_background: hsla(0.0, 0.0, 1.0, 1.0),
                scrollbar: hsla(0.0, 0.0, 0.70, 0.3),
                scrollbar_hover: hsla(0.0, 0.0, 0.60, 0.5),
                tab_active: hsla(0.0, 0.0, 1.0, 1.0),
                tab_inactive: hsla(0.0, 0.0, 0.96, 1.0),
                icon: hsla(0.0, 0.0, 0.30, 1.0),
                icon_muted: hsla(0.0, 0.0, 0.55, 1.0),
            },
            fonts: FontSettings {
                ui_family: SharedString::from("Inter"),
                mono_family: SharedString::from("JetBrains Mono"),
                ui_size: 13.0,
                mono_size: 13.0,
                editor_size: 14.0,
                line_height: 1.5,
            },
            spacing: Spacing {
                xs: 4.0,
                sm: 8.0,
                md: 12.0,
                lg: 16.0,
                xl: 24.0,
                panel_padding: 12.0,
                item_gap: 4.0,
                border_radius: 4.0,
            },
        }
    }
}

/// Global theme storage for runtime access
static CURRENT_THEME: parking_lot::RwLock<Option<Theme>> = parking_lot::RwLock::new(None);

/// Set the current theme globally
pub fn set_theme(theme: Theme) {
    *CURRENT_THEME.write() = Some(theme);
}

/// Get the current theme (or default dark theme)
pub fn current_theme() -> Theme {
    CURRENT_THEME
        .read()
        .clone()
        .unwrap_or_else(Theme::dark)
}
