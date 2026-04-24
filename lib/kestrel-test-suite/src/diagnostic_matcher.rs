//! Unified diagnostic type and two-way annotation matching.
//!
//! Collects diagnostics from all pipeline stages (lex, parse, infer, analyze)
//! into a single `TestDiagnostic` type, then checks them against inline
//! annotations in the source file.

use crate::annotation::{Annotation, AnnotationKind};

/// Severity of a test diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSeverity {
    Error,
    Warning,
    Info,
}

/// Unified diagnostic from any pipeline stage.
#[derive(Debug, Clone)]
pub struct TestDiagnostic {
    pub severity: TestSeverity,
    pub message: String,
    /// Analyzer descriptor ID (e.g. "E441"), if from an analyzer.
    pub code: Option<String>,
    /// 1-based line number.
    pub line: usize,
    /// File entity index.
    pub file_id: usize,
}

/// Convert codespan-reporting diagnostics to TestDiagnostics.
///
/// Resolves byte offsets to line numbers using the source text.
/// `sources` is a list of (file_id, source_text) pairs.
pub fn from_codespan_diagnostics(
    diagnostics: &[codespan_reporting::diagnostic::Diagnostic<usize>],
    sources: &[(usize, String)],
) -> Vec<TestDiagnostic> {
    let mut result = Vec::new();

    for diag in diagnostics {
        let severity = match diag.severity {
            codespan_reporting::diagnostic::Severity::Error
            | codespan_reporting::diagnostic::Severity::Bug => TestSeverity::Error,
            codespan_reporting::diagnostic::Severity::Warning => TestSeverity::Warning,
            codespan_reporting::diagnostic::Severity::Note
            | codespan_reporting::diagnostic::Severity::Help => TestSeverity::Info,
        };

        // Get line number from the primary label's byte range
        let (file_id, line) = diag
            .labels
            .iter()
            .find(|l| l.style == codespan_reporting::diagnostic::LabelStyle::Primary)
            .map(|label| {
                let line = byte_offset_to_line(label.file_id, label.range.start, sources);
                (label.file_id, line)
            })
            .unwrap_or((0, 0));

        // Build message: combine the main message with the primary label message
        let mut message = diag.message.clone();
        if let Some(label) = diag
            .labels
            .iter()
            .find(|l| l.style == codespan_reporting::diagnostic::LabelStyle::Primary)
        {
            if !label.message.is_empty() && label.message != diag.message {
                message = format!("{}: {}", message, label.message);
            }
        }

        result.push(TestDiagnostic {
            severity,
            message,
            code: None,
            line,
            file_id,
        });
    }

    result
}

/// Convert analyzer diagnostics to TestDiagnostics.
pub fn from_analyze_diagnostics(
    diagnostics: &[kestrel_analyze::AnalyzeDiagnostic],
) -> Vec<TestDiagnostic> {
    diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                kestrel_analyze::Severity::Error => TestSeverity::Error,
                kestrel_analyze::Severity::Warning => TestSeverity::Warning,
                kestrel_analyze::Severity::Info => TestSeverity::Info,
            };

            // Get line from primary label span
            let (file_id, line) = d
                .labels
                .iter()
                .find(|l| l.is_primary)
                .map(|l| (l.span.file_id, l.span.start))
                .unwrap_or((0, 0));

            TestDiagnostic {
                severity,
                message: d.message.clone(),
                code: Some(d.descriptor_id.to_string()),
                // Note: line here is a byte offset — callers must convert
                // using byte_offset_to_line if they have the source
                line,
                file_id,
            }
        })
        .collect()
}

/// Resolve byte offset to 1-based line number.
fn byte_offset_to_line(file_id: usize, byte_offset: usize, sources: &[(usize, String)]) -> usize {
    let source = sources
        .iter()
        .find(|(id, _)| *id == file_id)
        .map(|(_, s)| s.as_str())
        .unwrap_or("");

    // Count newlines before the byte offset
    source[..byte_offset.min(source.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1 // 1-based
}

/// Resolve analyzer diagnostics with proper line numbers from source text.
pub fn from_analyze_diagnostics_with_source(
    diagnostics: &[kestrel_analyze::AnalyzeDiagnostic],
    sources: &[(usize, String)],
) -> Vec<TestDiagnostic> {
    diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                kestrel_analyze::Severity::Error => TestSeverity::Error,
                kestrel_analyze::Severity::Warning => TestSeverity::Warning,
                kestrel_analyze::Severity::Info => TestSeverity::Info,
            };

            let (file_id, line) = d
                .labels
                .iter()
                .find(|l| l.is_primary)
                .map(|l| {
                    let line = byte_offset_to_line(l.span.file_id, l.span.start, sources);
                    (l.span.file_id, line)
                })
                .unwrap_or((0, 1));

            TestDiagnostic {
                severity,
                message: d.message.clone(),
                code: Some(d.descriptor_id.to_string()),
                line,
                file_id,
            }
        })
        .collect()
}

/// Two-way check: every annotation must match a diagnostic, and every
/// diagnostic on the test file must match an annotation.
///
/// Returns Ok(()) on success, Err with a detailed message on failure.
pub fn check(
    annotations: &[Annotation],
    diagnostics: &[TestDiagnostic],
    test_file_id: usize,
) -> Result<(), String> {
    // Filter diagnostics to only those from the test file
    let test_diags: Vec<&TestDiagnostic> = diagnostics
        .iter()
        .filter(|d| d.file_id == test_file_id)
        .collect();

    let mut errors = Vec::new();
    let mut matched_diags: Vec<bool> = vec![false; test_diags.len()];

    // Check 1: every annotation must have a matching diagnostic.
    // One annotation covers every matching diagnostic on its line — if the
    // compiler emits several legitimate errors on the same expression (e.g.
    // an arg type mismatch *and* a return type mismatch on the same call),
    // a single `// ERROR: type` shouldn't flag the extras as unexpected.
    for ann in annotations {
        let mut found = false;
        for (i, diag) in test_diags.iter().enumerate() {
            if matches_annotation(ann, diag) {
                found = true;
                matched_diags[i] = true;
            }
        }
        if !found {
            errors.push(format!(
                "  line {}: expected {} but no matching diagnostic found",
                ann.line,
                describe_annotation(ann),
            ));
        }
    }

    // Check 2: every test-file diagnostic must have a matching annotation
    for (i, diag) in test_diags.iter().enumerate() {
        if matched_diags[i] {
            continue;
        }
        // Only flag errors and warnings as unexpected
        if diag.severity == TestSeverity::Info {
            continue;
        }
        errors.push(format!(
            "  line {}: unexpected {}: {}{}",
            diag.line,
            match diag.severity {
                TestSeverity::Error => "error",
                TestSeverity::Warning => "warning",
                TestSeverity::Info => "info",
            },
            diag.message,
            diag.code
                .as_ref()
                .map(|c| format!(" [{}]", c))
                .unwrap_or_default(),
        ));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Diagnostic matching failed:\n{}",
            errors.join("\n")
        ))
    }
}

/// Check if a diagnostic matches an annotation.
fn matches_annotation(ann: &Annotation, diag: &TestDiagnostic) -> bool {
    // Must be on the same line
    if ann.line != diag.line {
        return false;
    }

    match &ann.kind {
        AnnotationKind::Error { message } => {
            if diag.severity != TestSeverity::Error {
                return false;
            }
            // If a message is specified, check substring (case-insensitive)
            message.as_ref().map_or(true, |msg| {
                diag.message.to_lowercase().contains(&msg.to_lowercase())
            })
        },
        AnnotationKind::ErrorCode { code } => {
            if diag.severity != TestSeverity::Error {
                return false;
            }
            diag.code.as_ref().is_some_and(|c| c == code)
        },
        AnnotationKind::Warning { message } => {
            if diag.severity != TestSeverity::Warning {
                return false;
            }
            message.as_ref().map_or(true, |msg| {
                diag.message.to_lowercase().contains(&msg.to_lowercase())
            })
        },
    }
}

/// Human-readable description of an annotation for error messages.
fn describe_annotation(ann: &Annotation) -> String {
    match &ann.kind {
        AnnotationKind::Error { message: Some(msg) } => format!("error containing '{}'", msg),
        AnnotationKind::Error { message: None } => "any error".to_string(),
        AnnotationKind::ErrorCode { code } => format!("error with code {}", code),
        AnnotationKind::Warning { message: Some(msg) } => format!("warning containing '{}'", msg),
        AnnotationKind::Warning { message: None } => "any warning".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_error_on_same_line() {
        let annotations = vec![Annotation {
            line: 3,
            kind: AnnotationKind::Error {
                message: Some("type mismatch".to_string()),
            },
        }];
        let diagnostics = vec![TestDiagnostic {
            severity: TestSeverity::Error,
            message: "type mismatch: expected Int64, got String".to_string(),
            code: None,
            line: 3,
            file_id: 0,
        }];
        assert!(check(&annotations, &diagnostics, 0).is_ok());
    }

    #[test]
    fn missing_expected_error() {
        let annotations = vec![Annotation {
            line: 3,
            kind: AnnotationKind::Error { message: None },
        }];
        let diagnostics = vec![];
        let result = check(&annotations, &diagnostics, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no matching diagnostic"));
    }

    #[test]
    fn unexpected_error() {
        let annotations = vec![];
        let diagnostics = vec![TestDiagnostic {
            severity: TestSeverity::Error,
            message: "unexpected error".to_string(),
            code: None,
            line: 5,
            file_id: 0,
        }];
        let result = check(&annotations, &diagnostics, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unexpected error"));
    }

    #[test]
    fn error_code_matching() {
        let annotations = vec![Annotation {
            line: 1,
            kind: AnnotationKind::ErrorCode {
                code: "E441".to_string(),
            },
        }];
        let diagnostics = vec![TestDiagnostic {
            severity: TestSeverity::Error,
            message: "type alias bounds in wrong context".to_string(),
            code: Some("E441".to_string()),
            line: 1,
            file_id: 0,
        }];
        assert!(check(&annotations, &diagnostics, 0).is_ok());
    }

    #[test]
    fn diagnostics_from_other_files_ignored() {
        let annotations = vec![];
        let diagnostics = vec![TestDiagnostic {
            severity: TestSeverity::Error,
            message: "stdlib error".to_string(),
            code: None,
            line: 100,
            file_id: 99, // different file
        }];
        // No test-file diagnostics, no annotations — passes
        assert!(check(&annotations, &diagnostics, 0).is_ok());
    }

    #[test]
    fn byte_offset_to_line_basic() {
        let sources = vec![(0, "line1\nline2\nline3".to_string())];
        assert_eq!(byte_offset_to_line(0, 0, &sources), 1); // start of line1
        assert_eq!(byte_offset_to_line(0, 6, &sources), 2); // start of line2
        assert_eq!(byte_offset_to_line(0, 12, &sources), 3); // start of line3
    }
}
