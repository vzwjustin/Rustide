//! Language Configuration
//!
//! Defines language-specific settings like LSP servers, DAP adapters,
//! comment styles, bracket pairs, and other editor behaviors.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a programming language.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageConfig {
    /// LSP server configuration.
    #[serde(default)]
    pub lsp: Option<LspConfig>,

    /// DAP adapter configuration.
    #[serde(default)]
    pub dap: Option<DapConfig>,

    /// Comment configuration.
    #[serde(default)]
    pub comments: CommentConfig,

    /// Bracket pairs for auto-closing and matching.
    #[serde(default)]
    pub brackets: Vec<BracketPair>,

    /// Auto-pair characters (e.g., quotes).
    #[serde(default)]
    pub auto_pairs: Vec<AutoPair>,

    /// Indentation settings.
    #[serde(default)]
    pub indentation: IndentationConfig,

    /// Word characters (for word selection/navigation).
    #[serde(default)]
    pub word_chars: Option<String>,

    /// File icon identifier.
    #[serde(default)]
    pub icon: Option<String>,

    /// Formatter command.
    #[serde(default)]
    pub formatter: Option<FormatterConfig>,

    /// Run configuration for executing code.
    #[serde(default)]
    pub run: Option<RunConfig>,

    /// Custom settings.
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

impl LanguageConfig {
    /// Create a new empty language configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for language configuration.
    pub fn builder() -> LanguageConfigBuilder {
        LanguageConfigBuilder::new()
    }
}

/// Builder for LanguageConfig.
#[derive(Debug, Default)]
pub struct LanguageConfigBuilder {
    config: LanguageConfig,
}

impl LanguageConfigBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set LSP configuration.
    pub fn lsp(mut self, lsp: LspConfig) -> Self {
        self.config.lsp = Some(lsp);
        self
    }

    /// Set DAP configuration.
    pub fn dap(mut self, dap: DapConfig) -> Self {
        self.config.dap = Some(dap);
        self
    }

    /// Set comment configuration.
    pub fn comments(mut self, comments: CommentConfig) -> Self {
        self.config.comments = comments;
        self
    }

    /// Add bracket pairs.
    pub fn brackets(mut self, brackets: Vec<BracketPair>) -> Self {
        self.config.brackets = brackets;
        self
    }

    /// Add auto-pairs.
    pub fn auto_pairs(mut self, auto_pairs: Vec<AutoPair>) -> Self {
        self.config.auto_pairs = auto_pairs;
        self
    }

    /// Set indentation configuration.
    pub fn indentation(mut self, indentation: IndentationConfig) -> Self {
        self.config.indentation = indentation;
        self
    }

    /// Set word characters.
    pub fn word_chars(mut self, chars: impl Into<String>) -> Self {
        self.config.word_chars = Some(chars.into());
        self
    }

    /// Set icon identifier.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.config.icon = Some(icon.into());
        self
    }

    /// Set formatter configuration.
    pub fn formatter(mut self, formatter: FormatterConfig) -> Self {
        self.config.formatter = Some(formatter);
        self
    }

    /// Set run configuration.
    pub fn run(mut self, run: RunConfig) -> Self {
        self.config.run = Some(run);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> LanguageConfig {
        self.config
    }
}

/// LSP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspConfig {
    /// Command to start the LSP server.
    pub command: String,

    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Working directory.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Initialization options.
    #[serde(default)]
    pub initialization_options: Option<serde_json::Value>,

    /// Server capabilities overrides.
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,

    /// Root URI patterns for workspace detection.
    #[serde(default)]
    pub root_patterns: Vec<String>,
}

impl LspConfig {
    /// Create a new LSP configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            initialization_options: None,
            settings: HashMap::new(),
            root_patterns: Vec::new(),
        }
    }

    /// Add arguments.
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Add environment variables.
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set initialization options.
    pub fn with_init_options(mut self, options: serde_json::Value) -> Self {
        self.initialization_options = Some(options);
        self
    }

    /// Add root patterns.
    pub fn with_root_patterns(mut self, patterns: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.root_patterns = patterns.into_iter().map(Into::into).collect();
        self
    }
}

/// DAP (Debug Adapter Protocol) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapConfig {
    /// Adapter identifier.
    pub adapter_id: String,

    /// Command to start the DAP adapter.
    pub command: String,

    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Port to connect to (for socket-based adapters).
    #[serde(default)]
    pub port: Option<u16>,

    /// Launch configuration templates.
    #[serde(default)]
    pub launch_templates: Vec<DapLaunchTemplate>,

    /// Attach configuration templates.
    #[serde(default)]
    pub attach_templates: Vec<DapLaunchTemplate>,
}

impl DapConfig {
    /// Create a new DAP configuration.
    pub fn new(adapter_id: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            port: None,
            launch_templates: Vec::new(),
            attach_templates: Vec::new(),
        }
    }

    /// Add arguments.
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Set port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Add launch template.
    pub fn with_launch_template(mut self, template: DapLaunchTemplate) -> Self {
        self.launch_templates.push(template);
        self
    }

    /// Add attach template.
    pub fn with_attach_template(mut self, template: DapLaunchTemplate) -> Self {
        self.attach_templates.push(template);
        self
    }
}

/// DAP launch/attach configuration template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapLaunchTemplate {
    /// Template name.
    pub name: String,

    /// Request type (launch or attach).
    pub request: String,

    /// Configuration values.
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

impl DapLaunchTemplate {
    /// Create a new launch template.
    pub fn launch(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            request: "launch".to_string(),
            config: HashMap::new(),
        }
    }

    /// Create a new attach template.
    pub fn attach(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            request: "attach".to_string(),
            config: HashMap::new(),
        }
    }

    /// Add configuration value.
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }
}

/// Comment configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentConfig {
    /// Line comment prefix (e.g., "//", "#").
    #[serde(default)]
    pub line: Option<String>,

    /// Block comment start (e.g., "/*").
    #[serde(default)]
    pub block_start: Option<String>,

    /// Block comment end (e.g., "*/").
    #[serde(default)]
    pub block_end: Option<String>,

    /// Documentation comment prefix (e.g., "///", "##").
    #[serde(default)]
    pub doc_line: Option<String>,

    /// Documentation block comment start (e.g., "/**").
    #[serde(default)]
    pub doc_block_start: Option<String>,

    /// Documentation block comment end (e.g., "*/").
    #[serde(default)]
    pub doc_block_end: Option<String>,
}

impl CommentConfig {
    /// Create a new comment configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set line comment.
    pub fn with_line(mut self, prefix: impl Into<String>) -> Self {
        self.line = Some(prefix.into());
        self
    }

    /// Set block comments.
    pub fn with_block(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.block_start = Some(start.into());
        self.block_end = Some(end.into());
        self
    }

    /// Set doc line comment.
    pub fn with_doc_line(mut self, prefix: impl Into<String>) -> Self {
        self.doc_line = Some(prefix.into());
        self
    }

    /// Set doc block comments.
    pub fn with_doc_block(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.doc_block_start = Some(start.into());
        self.doc_block_end = Some(end.into());
        self
    }

    /// Create C-style comments.
    pub fn c_style() -> Self {
        Self::new()
            .with_line("//")
            .with_block("/*", "*/")
    }

    /// Create shell-style comments.
    pub fn shell_style() -> Self {
        Self::new().with_line("#")
    }

    /// Create Rust-style comments.
    pub fn rust_style() -> Self {
        Self::new()
            .with_line("//")
            .with_block("/*", "*/")
            .with_doc_line("///")
            .with_doc_block("/**", "*/")
    }

    /// Create Python-style comments.
    pub fn python_style() -> Self {
        Self::new()
            .with_line("#")
            .with_block("\"\"\"", "\"\"\"")
            .with_doc_line("#")
            .with_doc_block("\"\"\"", "\"\"\"")
    }
}

/// A bracket pair for matching and auto-closing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketPair {
    /// Opening bracket.
    pub open: String,

    /// Closing bracket.
    pub close: String,

    /// Whether this pair should be auto-closed.
    #[serde(default = "default_true")]
    pub auto_close: bool,

    /// Whether this is a scope delimiter (for indentation).
    #[serde(default)]
    pub is_scope: bool,
}

fn default_true() -> bool {
    true
}

impl BracketPair {
    /// Create a new bracket pair.
    pub fn new(open: impl Into<String>, close: impl Into<String>) -> Self {
        Self {
            open: open.into(),
            close: close.into(),
            auto_close: true,
            is_scope: false,
        }
    }

    /// Set whether this pair auto-closes.
    pub fn auto_close(mut self, value: bool) -> Self {
        self.auto_close = value;
        self
    }

    /// Set whether this pair is a scope delimiter.
    pub fn is_scope(mut self, value: bool) -> Self {
        self.is_scope = value;
        self
    }

    /// Create standard bracket pairs.
    pub fn standard() -> Vec<Self> {
        vec![
            BracketPair::new("(", ")"),
            BracketPair::new("[", "]"),
            BracketPair::new("{", "}").is_scope(true),
        ]
    }

    /// Create angle bracket pair (for generics).
    pub fn angle() -> Self {
        BracketPair::new("<", ">").auto_close(false)
    }
}

/// An auto-pair configuration (e.g., for quotes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPair {
    /// The character that triggers auto-pairing.
    pub trigger: String,

    /// The character to insert after the cursor.
    pub pair: String,

    /// Characters that prevent auto-pairing when before cursor.
    #[serde(default)]
    pub not_before: Vec<String>,

    /// Characters that prevent auto-pairing when after cursor.
    #[serde(default)]
    pub not_after: Vec<String>,
}

impl AutoPair {
    /// Create a new auto-pair.
    pub fn new(trigger: impl Into<String>, pair: impl Into<String>) -> Self {
        Self {
            trigger: trigger.into(),
            pair: pair.into(),
            not_before: Vec::new(),
            not_after: Vec::new(),
        }
    }

    /// Create a symmetric auto-pair (same trigger and pair).
    pub fn symmetric(char: impl Into<String>) -> Self {
        let c = char.into();
        Self::new(c.clone(), c)
    }

    /// Set characters that prevent auto-pairing before cursor.
    pub fn not_before(mut self, chars: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.not_before = chars.into_iter().map(Into::into).collect();
        self
    }

    /// Set characters that prevent auto-pairing after cursor.
    pub fn not_after(mut self, chars: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.not_after = chars.into_iter().map(Into::into).collect();
        self
    }

    /// Create standard quote auto-pairs.
    pub fn quotes() -> Vec<Self> {
        vec![
            AutoPair::symmetric("\"").not_before(["\\", "\""]),
            AutoPair::symmetric("'").not_before(["\\", "'"]),
            AutoPair::symmetric("`").not_before(["\\", "`"]),
        ]
    }
}

/// Indentation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndentationConfig {
    /// Use tabs or spaces.
    #[serde(default)]
    pub use_tabs: bool,

    /// Tab width in spaces.
    #[serde(default = "default_tab_width")]
    pub tab_width: u32,

    /// Indent width (may differ from tab width).
    #[serde(default = "default_tab_width")]
    pub indent_width: u32,
}

fn default_tab_width() -> u32 {
    4
}

impl Default for IndentationConfig {
    fn default() -> Self {
        Self {
            use_tabs: false,
            tab_width: 4,
            indent_width: 4,
        }
    }
}

impl IndentationConfig {
    /// Create a new indentation configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Use tabs for indentation.
    pub fn with_tabs(mut self, width: u32) -> Self {
        self.use_tabs = true;
        self.tab_width = width;
        self.indent_width = width;
        self
    }

    /// Use spaces for indentation.
    pub fn with_spaces(mut self, width: u32) -> Self {
        self.use_tabs = false;
        self.indent_width = width;
        self
    }

    /// Get the indentation string.
    pub fn indent_string(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_width as usize)
        }
    }
}

/// Formatter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatterConfig {
    /// Command to run the formatter.
    pub command: String,

    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether the formatter reads from stdin.
    #[serde(default = "default_true")]
    pub stdin: bool,

    /// Whether to format on save.
    #[serde(default)]
    pub format_on_save: bool,
}

impl FormatterConfig {
    /// Create a new formatter configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            stdin: true,
            format_on_save: false,
        }
    }

    /// Add arguments.
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Set whether to read from stdin.
    pub fn with_stdin(mut self, stdin: bool) -> Self {
        self.stdin = stdin;
        self
    }

    /// Set whether to format on save.
    pub fn format_on_save(mut self, value: bool) -> Self {
        self.format_on_save = value;
        self
    }
}

/// Run configuration for executing code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    /// Command to run the code.
    pub command: String,

    /// Arguments for the command.
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Working directory.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Whether to use a build step first.
    #[serde(default)]
    pub build_first: bool,

    /// Build command (if build_first is true).
    #[serde(default)]
    pub build_command: Option<String>,

    /// Build arguments.
    #[serde(default)]
    pub build_args: Vec<String>,
}

impl RunConfig {
    /// Create a new run configuration.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            build_first: false,
            build_command: None,
            build_args: Vec::new(),
        }
    }

    /// Add arguments.
    pub fn with_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Set environment variables.
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Set working directory.
    pub fn with_cwd(mut self, cwd: impl Into<String>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Add build step.
    pub fn with_build(mut self, command: impl Into<String>, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.build_first = true;
        self.build_command = Some(command.into());
        self.build_args = args.into_iter().map(Into::into).collect();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_config_builder() {
        let config = LanguageConfig::builder()
            .comments(CommentConfig::rust_style())
            .brackets(BracketPair::standard())
            .build();

        assert!(config.comments.line.is_some());
        assert!(!config.brackets.is_empty());
    }

    #[test]
    fn test_lsp_config() {
        let lsp = LspConfig::new("rust-analyzer")
            .with_args(["--log-file", "/tmp/ra.log"])
            .with_root_patterns(["Cargo.toml"]);

        assert_eq!(lsp.command, "rust-analyzer");
        assert_eq!(lsp.args.len(), 2);
        assert_eq!(lsp.root_patterns.len(), 1);
    }

    #[test]
    fn test_dap_config() {
        let dap = DapConfig::new("codelldb", "codelldb")
            .with_args(["--port", "13000"])
            .with_port(13000);

        assert_eq!(dap.adapter_id, "codelldb");
        assert_eq!(dap.port, Some(13000));
    }

    #[test]
    fn test_comment_styles() {
        let c_style = CommentConfig::c_style();
        assert_eq!(c_style.line, Some("//".to_string()));
        assert_eq!(c_style.block_start, Some("/*".to_string()));

        let python_style = CommentConfig::python_style();
        assert_eq!(python_style.line, Some("#".to_string()));
    }

    #[test]
    fn test_bracket_pairs() {
        let brackets = BracketPair::standard();
        assert_eq!(brackets.len(), 3);

        let brace = &brackets[2];
        assert_eq!(brace.open, "{");
        assert!(brace.is_scope);
    }

    #[test]
    fn test_auto_pairs() {
        let quotes = AutoPair::quotes();
        assert_eq!(quotes.len(), 3);

        let double_quote = &quotes[0];
        assert_eq!(double_quote.trigger, "\"");
        assert_eq!(double_quote.pair, "\"");
    }

    #[test]
    fn test_indentation() {
        let spaces = IndentationConfig::new().with_spaces(2);
        assert!(!spaces.use_tabs);
        assert_eq!(spaces.indent_string(), "  ");

        let tabs = IndentationConfig::new().with_tabs(4);
        assert!(tabs.use_tabs);
        assert_eq!(tabs.indent_string(), "\t");
    }

    #[test]
    fn test_formatter_config() {
        let formatter = FormatterConfig::new("rustfmt")
            .with_args(["--edition", "2021"])
            .format_on_save(true);

        assert_eq!(formatter.command, "rustfmt");
        assert!(formatter.format_on_save);
    }

    #[test]
    fn test_run_config() {
        let run = RunConfig::new("cargo")
            .with_args(["run"])
            .with_build("cargo", ["build"]);

        assert!(run.build_first);
        assert_eq!(run.build_command, Some("cargo".to_string()));
    }
}
