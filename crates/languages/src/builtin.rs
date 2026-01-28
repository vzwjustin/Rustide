//! Built-in Language Definitions
//!
//! Provides pre-configured language definitions for common programming languages.

use crate::config::{
    AutoPair, BracketPair, CommentConfig, DapConfig, DapLaunchTemplate, FormatterConfig,
    IndentationConfig, LanguageConfig, LspConfig, RunConfig,
};
use crate::grammar::GrammarLoader;
use crate::registry::LanguageDefinition;

/// Get all built-in language definitions.
pub fn get_builtin_languages() -> Vec<LanguageDefinition> {
    let mut languages = Vec::new();

    // Add languages, logging errors for any that fail to load
    if let Some(lang) = get_rust_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_javascript_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_typescript_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_python_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_go_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_c_language() {
        languages.push(lang);
    }

    if let Some(lang) = get_json_language() {
        languages.push(lang);
    }

    // TODO: Re-enable when tree-sitter-toml-ng and tree-sitter-md cc version conflict is resolved
    // if let Some(lang) = get_toml_language() {
    //     languages.push(lang);
    // }

    // if let Some(lang) = get_markdown_language() {
    //     languages.push(lang);
    // }

    languages
}

/// Get the Rust language definition.
pub fn get_rust_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::rust().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("rust-analyzer")
                .with_root_patterns(["Cargo.toml", "rust-project.json"]),
        )
        .dap(
            DapConfig::new("codelldb", "codelldb")
                .with_args(["--port", "13000"])
                .with_port(13000)
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("lldb"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("program", serde_json::json!("${workspaceFolder}/target/debug/${workspaceFolderBasename}"))
                        .with_config("cwd", serde_json::json!("${workspaceFolder}")),
                ),
        )
        .comments(CommentConfig::rust_style())
        .brackets(vec![
            BracketPair::new("(", ")"),
            BracketPair::new("[", "]"),
            BracketPair::new("{", "}").is_scope(true),
            BracketPair::angle(),
        ])
        .auto_pairs(AutoPair::quotes())
        .indentation(IndentationConfig::new().with_spaces(4))
        .icon("rust")
        .formatter(
            FormatterConfig::new("rustfmt")
                .with_args(["--edition", "2021"])
                .format_on_save(true),
        )
        .run(
            RunConfig::new("cargo")
                .with_args(["run"])
                .with_build("cargo", ["build"]),
        )
        .build();

    Some(
        LanguageDefinition::builder("rust", "Rust")
            .extensions(["rs"])
            .file_names(["Cargo.toml", "Cargo.lock"])
            .mime_types(["text/x-rust"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the JavaScript language definition.
pub fn get_javascript_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::javascript().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("typescript-language-server")
                .with_args(["--stdio"])
                .with_root_patterns(["package.json", "jsconfig.json"]),
        )
        .dap(
            DapConfig::new("node", "node")
                .with_args(["--inspect"])
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("node"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("program", serde_json::json!("${file}")),
                ),
        )
        .comments(CommentConfig::c_style())
        .brackets(BracketPair::standard())
        .auto_pairs(AutoPair::quotes())
        .indentation(IndentationConfig::new().with_spaces(2))
        .icon("javascript")
        .formatter(
            FormatterConfig::new("prettier")
                .with_args(["--stdin-filepath", "${file}"])
                .format_on_save(true),
        )
        .run(RunConfig::new("node").with_args(["${file}"]))
        .build();

    Some(
        LanguageDefinition::builder("javascript", "JavaScript")
            .extensions(["js", "mjs", "cjs", "jsx"])
            .file_names(["package.json", "jsconfig.json"])
            .mime_types(["text/javascript", "application/javascript"])
            .first_line_patterns(["#!/usr/bin/env node"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the TypeScript language definition.
///
/// Note: TypeScript uses the JavaScript grammar with additional type annotations.
/// For full TypeScript support, a dedicated TypeScript grammar would be needed.
pub fn get_typescript_language() -> Option<LanguageDefinition> {
    // Use JavaScript grammar for now (TypeScript is a superset)
    let grammar = GrammarLoader::javascript().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("typescript-language-server")
                .with_args(["--stdio"])
                .with_root_patterns(["tsconfig.json", "package.json"]),
        )
        .dap(
            DapConfig::new("node", "ts-node")
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("node"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("program", serde_json::json!("${file}"))),
        )
        .comments(CommentConfig::c_style())
        .brackets(vec![
            BracketPair::new("(", ")"),
            BracketPair::new("[", "]"),
            BracketPair::new("{", "}").is_scope(true),
            BracketPair::angle(),
        ])
        .auto_pairs(AutoPair::quotes())
        .indentation(IndentationConfig::new().with_spaces(2))
        .icon("typescript")
        .formatter(
            FormatterConfig::new("prettier")
                .with_args(["--stdin-filepath", "${file}"])
                .format_on_save(true),
        )
        .run(RunConfig::new("npx").with_args(["ts-node", "${file}"]))
        .build();

    Some(
        LanguageDefinition::builder("typescript", "TypeScript")
            .extensions(["ts", "mts", "cts", "tsx"])
            .file_names(["tsconfig.json"])
            .mime_types(["text/typescript", "application/typescript"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the Python language definition.
pub fn get_python_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::python().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("pyright-langserver")
                .with_args(["--stdio"])
                .with_root_patterns(["pyproject.toml", "setup.py", "requirements.txt", "pyrightconfig.json"]),
        )
        .dap(
            DapConfig::new("debugpy", "python")
                .with_args(["-m", "debugpy.adapter"])
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("python"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("program", serde_json::json!("${file}")),
                ),
        )
        .comments(CommentConfig::python_style())
        .brackets(vec![
            BracketPair::new("(", ")"),
            BracketPair::new("[", "]"),
            BracketPair::new("{", "}"),
        ])
        .auto_pairs(vec![
            AutoPair::symmetric("\"").not_before(["\\", "\""]),
            AutoPair::symmetric("'").not_before(["\\", "'"]),
            AutoPair::new("\"\"\"", "\"\"\""),
            AutoPair::new("'''", "'''"),
        ])
        .indentation(IndentationConfig::new().with_spaces(4))
        .icon("python")
        .formatter(
            FormatterConfig::new("black")
                .with_args(["-"])
                .format_on_save(true),
        )
        .run(RunConfig::new("python").with_args(["${file}"]))
        .build();

    Some(
        LanguageDefinition::builder("python", "Python")
            .extensions(["py", "pyi", "pyw"])
            .file_names(["pyproject.toml", "setup.py", "requirements.txt"])
            .mime_types(["text/x-python"])
            .first_line_patterns(["#!/usr/bin/env python", "#!/usr/bin/python"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the Go language definition.
pub fn get_go_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::go().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("gopls")
                .with_root_patterns(["go.mod", "go.sum"]),
        )
        .dap(
            DapConfig::new("delve", "dlv")
                .with_args(["dap"])
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("go"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("mode", serde_json::json!("debug"))
                        .with_config("program", serde_json::json!("${workspaceFolder}")),
                ),
        )
        .comments(CommentConfig::c_style())
        .brackets(BracketPair::standard())
        .auto_pairs(vec![
            AutoPair::symmetric("\"").not_before(["\\", "\""]),
            AutoPair::symmetric("'").not_before(["\\", "'"]),
            AutoPair::symmetric("`"),
        ])
        .indentation(IndentationConfig::new().with_tabs(4))
        .icon("go")
        .formatter(
            FormatterConfig::new("gofmt")
                .format_on_save(true),
        )
        .run(
            RunConfig::new("go")
                .with_args(["run", "${file}"]),
        )
        .build();

    Some(
        LanguageDefinition::builder("go", "Go")
            .extensions(["go"])
            .file_names(["go.mod", "go.sum"])
            .mime_types(["text/x-go"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the C language definition.
pub fn get_c_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::c().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("clangd")
                .with_root_patterns(["compile_commands.json", "CMakeLists.txt", "Makefile"]),
        )
        .dap(
            DapConfig::new("cppdbg", "gdb")
                .with_args(["--interpreter=mi"])
                .with_launch_template(
                    DapLaunchTemplate::launch("Launch")
                        .with_config("type", serde_json::json!("cppdbg"))
                        .with_config("request", serde_json::json!("launch"))
                        .with_config("program", serde_json::json!("${workspaceFolder}/a.out"))
                        .with_config("MIMode", serde_json::json!("gdb")),
                ),
        )
        .comments(CommentConfig::c_style())
        .brackets(BracketPair::standard())
        .auto_pairs(AutoPair::quotes())
        .indentation(IndentationConfig::new().with_spaces(4))
        .icon("c")
        .formatter(
            FormatterConfig::new("clang-format")
                .with_args(["--assume-filename=${file}"])
                .format_on_save(true),
        )
        .run(
            RunConfig::new("./a.out")
                .with_build("gcc", ["-g", "${file}", "-o", "a.out"]),
        )
        .build();

    Some(
        LanguageDefinition::builder("c", "C")
            .extensions(["c", "h"])
            .file_names(["Makefile", "CMakeLists.txt"])
            .mime_types(["text/x-c"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

/// Get the JSON language definition.
pub fn get_json_language() -> Option<LanguageDefinition> {
    let grammar = GrammarLoader::json().ok()?;

    let config = LanguageConfig::builder()
        .lsp(
            LspConfig::new("vscode-json-language-server")
                .with_args(["--stdio"]),
        )
        .comments(CommentConfig::new()) // JSON doesn't support comments
        .brackets(vec![
            BracketPair::new("[", "]"),
            BracketPair::new("{", "}").is_scope(true),
        ])
        .auto_pairs(vec![AutoPair::symmetric("\"").not_before(["\\", "\""])])
        .indentation(IndentationConfig::new().with_spaces(2))
        .icon("json")
        .formatter(
            FormatterConfig::new("prettier")
                .with_args(["--stdin-filepath", "${file}"])
                .format_on_save(true),
        )
        .build();

    Some(
        LanguageDefinition::builder("json", "JSON")
            .extensions(["json", "jsonc", "json5"])
            .file_names(["package.json", "tsconfig.json", ".prettierrc"])
            .mime_types(["application/json"])
            .grammar(grammar)
            .config(config)
            .build(),
    )
}

// TODO: Re-enable when tree-sitter-toml-ng and tree-sitter-md cc version conflict is resolved
// /// Get the TOML language definition.
// pub fn get_toml_language() -> Option<LanguageDefinition> {
//     let grammar = GrammarLoader::toml().ok()?;
//
//     let config = LanguageConfig::builder()
//         .lsp(
//             LspConfig::new("taplo")
//                 .with_args(["lsp", "stdio"]),
//         )
//         .comments(CommentConfig::shell_style())
//         .brackets(vec![
//             BracketPair::new("[", "]"),
//             BracketPair::new("{", "}"),
//         ])
//         .auto_pairs(vec![
//             AutoPair::symmetric("\"").not_before(["\\", "\""]),
//             AutoPair::symmetric("'").not_before(["\\", "'"]),
//             AutoPair::new("\"\"\"", "\"\"\""),
//             AutoPair::new("'''", "'''"),
//         ])
//         .indentation(IndentationConfig::new().with_spaces(2))
//         .icon("toml")
//         .formatter(
//             FormatterConfig::new("taplo")
//                 .with_args(["fmt", "-"]),
//         )
//         .build();
//
//     Some(
//         LanguageDefinition::builder("toml", "TOML")
//             .extensions(["toml"])
//             .file_names(["Cargo.toml", "pyproject.toml", ".taplo.toml"])
//             .mime_types(["application/toml"])
//             .grammar(grammar)
//             .config(config)
//             .build(),
//     )
// }
//
// /// Get the Markdown language definition.
// pub fn get_markdown_language() -> Option<LanguageDefinition> {
//     let grammar = GrammarLoader::markdown().ok()?;
//
//     let config = LanguageConfig::builder()
//         .lsp(
//             LspConfig::new("marksman")
//                 .with_root_patterns([".marksman.toml"]),
//         )
//         .comments(CommentConfig::new().with_block("<!--", "-->"))
//         .brackets(vec![
//             BracketPair::new("(", ")"),
//             BracketPair::new("[", "]"),
//             BracketPair::new("{", "}"),
//             BracketPair::new("<", ">"),
//         ])
//         .auto_pairs(vec![
//             AutoPair::symmetric("*"),
//             AutoPair::symmetric("_"),
//             AutoPair::symmetric("`"),
//             AutoPair::symmetric("\""),
//         ])
//         .indentation(IndentationConfig::new().with_spaces(2))
//         .icon("markdown")
//         .formatter(
//             FormatterConfig::new("prettier")
//                 .with_args(["--stdin-filepath", "${file}"])
//                 .format_on_save(false),
//         )
//         .build();
//
//     Some(
//         LanguageDefinition::builder("markdown", "Markdown")
//             .extensions(["md", "markdown", "mdown", "mkd"])
//             .file_names(["README.md", "CHANGELOG.md", "LICENSE.md"])
//             .mime_types(["text/markdown"])
//             .grammar(grammar)
//             .config(config)
//             .build(),
//     )
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_languages() {
        let languages = get_builtin_languages();
        assert!(!languages.is_empty());
    }

    #[test]
    fn test_rust_language() {
        let rust = get_rust_language();
        assert!(rust.is_some());

        let rust = rust.unwrap();
        assert_eq!(rust.id.as_str(), "rust");
        assert!(rust.matches_extension("rs"));
        assert!(rust.config.lsp.is_some());
        assert!(rust.config.dap.is_some());
    }

    #[test]
    fn test_javascript_language() {
        let js = get_javascript_language();
        assert!(js.is_some());

        let js = js.unwrap();
        assert_eq!(js.id.as_str(), "javascript");
        assert!(js.matches_extension("js"));
        assert!(js.matches_extension("jsx"));
    }

    #[test]
    fn test_typescript_language() {
        let ts = get_typescript_language();
        assert!(ts.is_some());

        let ts = ts.unwrap();
        assert_eq!(ts.id.as_str(), "typescript");
        assert!(ts.matches_extension("ts"));
        assert!(ts.matches_extension("tsx"));
    }

    #[test]
    fn test_python_language() {
        let py = get_python_language();
        assert!(py.is_some());

        let py = py.unwrap();
        assert_eq!(py.id.as_str(), "python");
        assert!(py.matches_extension("py"));
        assert!(py.matches_first_line("#!/usr/bin/env python"));
    }

    #[test]
    fn test_go_language() {
        let go = get_go_language();
        assert!(go.is_some());

        let go = go.unwrap();
        assert_eq!(go.id.as_str(), "go");
        assert!(go.matches_extension("go"));
    }

    #[test]
    fn test_c_language() {
        let c = get_c_language();
        assert!(c.is_some());

        let c = c.unwrap();
        assert_eq!(c.id.as_str(), "c");
        assert!(c.matches_extension("c"));
        assert!(c.matches_extension("h"));
    }

    #[test]
    fn test_json_language() {
        let json = get_json_language();
        assert!(json.is_some());

        let json = json.unwrap();
        assert_eq!(json.id.as_str(), "json");
        assert!(json.matches_extension("json"));
    }

    // TODO: Re-enable when tree-sitter-toml-ng and tree-sitter-md cc version conflict is resolved
    // #[test]
    // fn test_toml_language() {
    //     let toml = get_toml_language();
    //     assert!(toml.is_some());
    //
    //     let toml = toml.unwrap();
    //     assert_eq!(toml.id.as_str(), "toml");
    //     assert!(toml.matches_extension("toml"));
    // }
    //
    // #[test]
    // fn test_markdown_language() {
    //     let md = get_markdown_language();
    //     assert!(md.is_some());
    //
    //     let md = md.unwrap();
    //     assert_eq!(md.id.as_str(), "markdown");
    //     assert!(md.matches_extension("md"));
    // }

    #[test]
    fn test_language_configs_have_lsp() {
        let languages = get_builtin_languages();
        for lang in &languages {
            if lang.id.as_str() != "markdown" {
                // markdown might not always have LSP
                assert!(
                    lang.config.lsp.is_some(),
                    "Language {} should have LSP config",
                    lang.id
                );
            }
        }
    }

    #[test]
    fn test_all_languages_have_grammar() {
        let languages = get_builtin_languages();
        for lang in &languages {
            // Grammar is required, so if we got here it has one
            let parser = lang.grammar.create_parser();
            assert!(
                parser.is_ok(),
                "Language {} should have working parser",
                lang.id
            );
        }
    }
}
