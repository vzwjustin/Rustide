//! Task output handling

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Type of output
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputType {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
    /// System message
    System,
}

/// A line of output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLine {
    /// Type of output
    pub output_type: OutputType,
    /// The text content
    pub text: String,
    /// Timestamp (unix millis)
    pub timestamp: u64,
}

impl OutputLine {
    /// Create a new output line
    pub fn new(output_type: OutputType, text: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            output_type,
            text: text.to_string(),
            timestamp,
        }
    }

    /// Create a stdout line
    pub fn stdout(text: &str) -> Self {
        Self::new(OutputType::Stdout, text)
    }

    /// Create a stderr line
    pub fn stderr(text: &str) -> Self {
        Self::new(OutputType::Stderr, text)
    }

    /// Create a system message
    pub fn system(text: &str) -> Self {
        Self::new(OutputType::System, text)
    }

    /// Check if this is an error line
    pub fn is_error(&self) -> bool {
        self.output_type == OutputType::Stderr
    }
}

/// ANSI color codes
pub mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const RED: &str = "\x1b[31m";
    pub const GREEN: &str = "\x1b[32m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const BLUE: &str = "\x1b[34m";
    pub const MAGENTA: &str = "\x1b[35m";
    pub const CYAN: &str = "\x1b[36m";
    pub const WHITE: &str = "\x1b[37m";
}

/// Parsed ANSI segment
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiSegment {
    pub text: String,
    pub bold: bool,
    pub dim: bool,
    pub foreground: Option<AnsiColor>,
}

/// ANSI colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

/// Output parser for processing task output
pub struct OutputParser {
    /// Buffer for incomplete lines
    buffer: String,
    /// Current ANSI state
    current_bold: bool,
    current_dim: bool,
    current_fg: Option<AnsiColor>,
}

impl OutputParser {
    /// Create a new output parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            current_bold: false,
            current_dim: false,
            current_fg: None,
        }
    }

    /// Parse output and return complete lines
    pub fn parse(&mut self, data: &str) -> Vec<String> {
        self.buffer.push_str(data);

        let mut lines = Vec::new();
        while let Some(pos) = self.buffer.find('\n') {
            let line = self.buffer[..pos].to_string();
            self.buffer = self.buffer[pos + 1..].to_string();

            // Handle \r\n
            let line = line.trim_end_matches('\r').to_string();
            lines.push(line);
        }

        lines
    }

    /// Flush any remaining buffer content
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(std::mem::take(&mut self.buffer))
        }
    }

    /// Strip ANSI codes from text
    pub fn strip_ansi(text: &str) -> String {
        let mut result = String::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip ANSI escape sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Parse ANSI codes into segments
    pub fn parse_ansi(text: &str) -> Vec<AnsiSegment> {
        let mut segments = Vec::new();
        let mut current_text = String::new();
        let mut bold = false;
        let mut dim = false;
        let mut fg: Option<AnsiColor> = None;

        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Save current segment if non-empty
                if !current_text.is_empty() {
                    segments.push(AnsiSegment {
                        text: std::mem::take(&mut current_text),
                        bold,
                        dim,
                        foreground: fg,
                    });
                }

                // Parse ANSI sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['

                    let mut code = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_ascii_alphabetic() {
                            chars.next();
                            break;
                        }
                        code.push(chars.next().unwrap());
                    }

                    // Parse codes
                    for part in code.split(';') {
                        match part {
                            "0" => {
                                bold = false;
                                dim = false;
                                fg = None;
                            }
                            "1" => bold = true,
                            "2" => dim = true,
                            "30" => fg = Some(AnsiColor::Black),
                            "31" => fg = Some(AnsiColor::Red),
                            "32" => fg = Some(AnsiColor::Green),
                            "33" => fg = Some(AnsiColor::Yellow),
                            "34" => fg = Some(AnsiColor::Blue),
                            "35" => fg = Some(AnsiColor::Magenta),
                            "36" => fg = Some(AnsiColor::Cyan),
                            "37" => fg = Some(AnsiColor::White),
                            _ => {}
                        }
                    }
                }
            } else {
                current_text.push(c);
            }
        }

        // Save remaining text
        if !current_text.is_empty() {
            segments.push(AnsiSegment {
                text: current_text,
                bold,
                dim,
                foreground: fg,
            });
        }

        segments
    }

    /// Check if text contains ANSI codes
    pub fn has_ansi(text: &str) -> bool {
        text.contains('\x1b')
    }
}

impl Default for OutputParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_line_creation() {
        let line = OutputLine::stdout("hello");
        assert_eq!(line.text, "hello");
        assert_eq!(line.output_type, OutputType::Stdout);
    }

    #[test]
    fn test_output_line_stderr() {
        let line = OutputLine::stderr("error");
        assert!(line.is_error());
    }

    #[test]
    fn test_output_parser_lines() {
        let mut parser = OutputParser::new();
        let lines = parser.parse("line1\nline2\nline3\n");
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_output_parser_partial() {
        let mut parser = OutputParser::new();

        let lines1 = parser.parse("hello ");
        assert!(lines1.is_empty());

        let lines2 = parser.parse("world\n");
        assert_eq!(lines2, vec!["hello world"]);
    }

    #[test]
    fn test_strip_ansi() {
        let text = "\x1b[31mred\x1b[0m normal";
        let stripped = OutputParser::strip_ansi(text);
        assert_eq!(stripped, "red normal");
    }

    #[test]
    fn test_parse_ansi() {
        let text = "\x1b[31mred\x1b[0m";
        let segments = OutputParser::parse_ansi(text);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "red");
        assert_eq!(segments[0].foreground, Some(AnsiColor::Red));
    }

    #[test]
    fn test_has_ansi() {
        assert!(OutputParser::has_ansi("\x1b[31mred\x1b[0m"));
        assert!(!OutputParser::has_ansi("plain text"));
    }

    #[test]
    fn test_crlf_handling() {
        let mut parser = OutputParser::new();
        let lines = parser.parse("line1\r\nline2\r\n");
        assert_eq!(lines, vec!["line1", "line2"]);
    }
}
