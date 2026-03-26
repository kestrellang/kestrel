//! Parse file headers and inline annotations from `.ks` test files.
//!
//! Headers are `// key: value` lines at the top of the file.
//! Inline annotations are `// ERROR`, `// WARN` comments at end of source lines.

use std::str::FromStr;

/// Which pipeline stage this test targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    /// Check diagnostics (errors/warnings) against inline annotations.
    Diagnostics,
    /// Lower to MIR and compare against a golden snapshot file.
    Mir,
    /// Compile, link, run, and check stdout/exit code.
    Execution,
}

/// Configuration parsed from file headers.
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub test_mode: TestMode,
    pub stdlib: bool,
    /// Extra .ks files to include (relative to the test file's directory).
    pub include: Vec<String>,
    pub skip: Option<String>,
    pub expect_exit: Option<i32>,
    pub expect_stdout: Option<String>,
    pub stdout_contains: Option<String>,
    pub mir_snapshot: Option<String>,
    pub mir_filter: Option<String>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_mode: TestMode::Diagnostics,
            stdlib: true,
            include: Vec::new(),
            skip: None,
            expect_exit: None,
            expect_stdout: None,
            stdout_contains: None,
            mir_snapshot: None,
            mir_filter: None,
        }
    }
}

/// An inline annotation on a specific source line.
#[derive(Debug, Clone)]
pub struct Annotation {
    /// 1-based line number in the source file.
    pub line: usize,
    pub kind: AnnotationKind,
}

/// What kind of diagnostic is expected.
#[derive(Debug, Clone)]
pub enum AnnotationKind {
    /// Any error, optionally matching a message substring.
    Error { message: Option<String> },
    /// Error with a specific analyzer descriptor code (e.g. "E441").
    ErrorCode { code: String },
    /// Any warning, optionally matching a message substring.
    Warning { message: Option<String> },
}

/// Parse the header config from a source file.
///
/// Headers are `// key: value` lines at the top of the file,
/// before any non-comment, non-blank line.
pub fn parse_test_config(source: &str) -> TestConfig {
    let mut config = TestConfig::default();

    for line in source.lines() {
        let trimmed = line.trim();
        // Stop at first non-comment, non-blank line
        if !trimmed.starts_with("//") && !trimmed.is_empty() {
            break;
        }
        // Parse `// key: value` headers
        let Some(comment) = trimmed.strip_prefix("//") else {
            continue;
        };
        let comment = comment.trim();
        let Some((key, value)) = comment.split_once(':') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();

        match key.as_str() {
            "test" => {
                config.test_mode = match value.to_lowercase().as_str() {
                    "diagnostics" => TestMode::Diagnostics,
                    "mir" => TestMode::Mir,
                    "execution" => TestMode::Execution,
                    _ => TestMode::Diagnostics,
                };
            }
            "stdlib" => {
                config.stdlib = value.to_lowercase() != "false";
            }
            "include" => {
                config.include.push(value.trim().to_string());
            }
            "skip" => {
                config.skip = Some(value.to_string());
            }
            "expect-exit" => {
                config.expect_exit = i32::from_str(value).ok();
            }
            "expect-stdout" => {
                config.expect_stdout = Some(value.to_string());
            }
            "stdout-contains" => {
                config.stdout_contains = Some(value.to_string());
            }
            "mir-snapshot" => {
                config.mir_snapshot = Some(value.to_string());
            }
            "mir-filter" => {
                config.mir_filter = Some(value.to_string());
            }
            _ => {}
        }
    }

    config
}

/// Parse inline annotations from source.
///
/// Scans every line for `// ERROR`, `// WARN` end-of-line comments.
/// Supports:
/// - `// ERROR` — any error on this line
/// - `// ERROR: message` — error with message substring
/// - `// ERROR(E441)` — error with specific analyzer code
/// - `// WARN` / `// WARN: message` — warnings
pub fn parse_annotations(source: &str) -> Vec<Annotation> {
    let mut annotations = Vec::new();

    for (idx, line) in source.lines().enumerate() {
        let line_num = idx + 1; // 1-based

        // Find the last `//` that's not inside a string literal.
        // Simple heuristic: find `// ERROR` or `// WARN` anywhere in the line.
        if let Some(annotation) = try_parse_annotation(line, line_num) {
            annotations.push(annotation);
        }
    }

    annotations
}

/// Try to parse an annotation from a single line.
fn try_parse_annotation(line: &str, line_num: usize) -> Option<Annotation> {
    // Look for `// ERROR` or `// WARN` patterns in the line.
    // We search for the pattern rather than just any `//` to avoid
    // matching regular comments.
    let comment = find_annotation_comment(line)?;
    let comment = comment.trim();

    // Try `ERROR(Exxxx)` pattern first
    if let Some(rest) = comment.strip_prefix("ERROR(") {
        if let Some(code) = rest.strip_suffix(')') {
            return Some(Annotation {
                line: line_num,
                kind: AnnotationKind::ErrorCode {
                    code: code.trim().to_string(),
                },
            });
        }
    }

    // Try `ERROR: message` or bare `ERROR`
    if let Some(rest) = comment.strip_prefix("ERROR") {
        let message = rest
            .strip_prefix(':')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        return Some(Annotation {
            line: line_num,
            kind: AnnotationKind::Error { message },
        });
    }

    // Try `WARN: message` or bare `WARN`
    if let Some(rest) = comment.strip_prefix("WARN") {
        let message = rest
            .strip_prefix(':')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        return Some(Annotation {
            line: line_num,
            kind: AnnotationKind::Warning { message },
        });
    }

    None
}

/// Find the annotation comment portion of a line.
///
/// Searches for `// ERROR` or `// WARN` patterns (case-sensitive).
/// Returns the text after `//` if found.
fn find_annotation_comment(line: &str) -> Option<&str> {
    // Find all `//` positions and check if followed by ERROR or WARN
    let mut search_from = 0;
    while let Some(pos) = line[search_from..].find("//") {
        let abs_pos = search_from + pos;
        let after_slashes = &line[abs_pos + 2..].trim_start();
        if after_slashes.starts_with("ERROR") || after_slashes.starts_with("WARN") {
            return Some(after_slashes);
        }
        search_from = abs_pos + 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_config() {
        let config = parse_test_config("module Test\nfunc main() {}");
        assert_eq!(config.test_mode, TestMode::Diagnostics);
        assert!(config.stdlib);
        assert!(config.skip.is_none());
    }

    #[test]
    fn parse_full_config() {
        let source = r#"// test: execution
// stdlib: false
// expect-exit: 42
// expect-stdout: hello world
// skip: codegen incomplete

module Test
"#;
        let config = parse_test_config(source);
        assert_eq!(config.test_mode, TestMode::Execution);
        assert!(!config.stdlib);
        assert_eq!(config.expect_exit, Some(42));
        assert_eq!(config.expect_stdout.as_deref(), Some("hello world"));
        assert_eq!(config.skip.as_deref(), Some("codegen incomplete"));
    }

    #[test]
    fn parse_error_annotation() {
        let source = "let x: Int = \"hi\" // ERROR: type mismatch";
        let annotations = parse_annotations(source);
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].line, 1);
        match &annotations[0].kind {
            AnnotationKind::Error { message } => {
                assert_eq!(message.as_deref(), Some("type mismatch"));
            }
            _ => panic!("expected Error annotation"),
        }
    }

    #[test]
    fn parse_error_code_annotation() {
        let source = "type Foo = Int // ERROR(E441)";
        let annotations = parse_annotations(source);
        assert_eq!(annotations.len(), 1);
        match &annotations[0].kind {
            AnnotationKind::ErrorCode { code } => assert_eq!(code, "E441"),
            _ => panic!("expected ErrorCode annotation"),
        }
    }

    #[test]
    fn parse_bare_error() {
        let source = "let x = bad // ERROR";
        let annotations = parse_annotations(source);
        assert_eq!(annotations.len(), 1);
        match &annotations[0].kind {
            AnnotationKind::Error { message } => assert!(message.is_none()),
            _ => panic!("expected Error annotation"),
        }
    }

    #[test]
    fn parse_warn_annotation() {
        let source = "let x = 5 // WARN: unused variable";
        let annotations = parse_annotations(source);
        assert_eq!(annotations.len(), 1);
        match &annotations[0].kind {
            AnnotationKind::Warning { message } => {
                assert_eq!(message.as_deref(), Some("unused variable"));
            }
            _ => panic!("expected Warning annotation"),
        }
    }

    #[test]
    fn regular_comments_not_matched() {
        let source = "// This is a normal comment about ERROR handling";
        let annotations = parse_annotations(source);
        // "ERROR" appears in the comment but not at the start of the
        // comment text after `//`. The parser only triggers when the
        // comment text starts with ERROR or WARN.
        assert_eq!(annotations.len(), 0);
    }

    #[test]
    fn no_annotations() {
        let source = "let x = 5 // just a comment\nlet y = 10";
        let annotations = parse_annotations(source);
        assert_eq!(annotations.len(), 0);
    }
}
