//! Diagnostic and position conversion utilities.
//!
//! Converts between Kestrel compiler types and LSP format.

use kestrel_reporting::{Diagnostic, Severity};
use tower_lsp::lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity, Position, Range};

/// Convert an LSP Position (line/column) to a byte offset.
///
/// LSP uses 0-based line and column numbers.
pub fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let mut current_line = 0u32;

    for (i, ch) in source.char_indices() {
        if current_line == position.line {
            // We're on the target line, count columns
            let mut col = 0u32;
            for (j, c) in source[i..].char_indices() {
                if col == position.character {
                    return i + j;
                }
                if c == '\n' {
                    // Position is beyond end of line
                    return i + j;
                }
                col += 1;
            }
            // Position is at or beyond end of file
            return source.len();
        }
        if ch == '\n' {
            current_line += 1;
        }
    }

    // Position is beyond end of file
    source.len()
}

/// Convert a byte offset to an LSP Position (line/column).
///
/// LSP uses 0-based line and column numbers.
pub fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    Position::new(line, col)
}

/// Convert a byte range to an LSP Range.
pub fn byte_range_to_range(source: &str, start: usize, end: usize) -> Range {
    Range::new(
        byte_offset_to_position(source, start),
        byte_offset_to_position(source, end),
    )
}

/// Convert codespan Severity to LSP DiagnosticSeverity.
pub fn convert_severity(severity: Severity) -> DiagnosticSeverity {
    match severity {
        Severity::Bug | Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
        Severity::Note => DiagnosticSeverity::INFORMATION,
        Severity::Help => DiagnosticSeverity::HINT,
    }
}

/// Convert a codespan Diagnostic to an LSP Diagnostic.
///
/// Returns None if the diagnostic has no labels (no location info).
pub fn convert_diagnostic(diagnostic: &Diagnostic<usize>, source: &str) -> Option<LspDiagnostic> {
    // Get the primary label's range, or return None if no labels
    let primary_label = diagnostic.labels.first()?;
    let range = byte_range_to_range(source, primary_label.range.start, primary_label.range.end);

    // Build the message, including label messages if present
    let mut message = diagnostic.message.clone();
    if !primary_label.message.is_empty() && primary_label.message != diagnostic.message {
        message = format!("{}\n{}", message, primary_label.message);
    }

    // Add notes to the message
    for note in &diagnostic.notes {
        message = format!("{}\n\nnote: {}", message, note);
    }

    Some(LspDiagnostic {
        range,
        severity: Some(convert_severity(diagnostic.severity)),
        code: None,
        code_description: None,
        source: Some("kestrel".to_string()),
        message,
        related_information: None,
        tags: None,
        data: None,
    })
}

/// Convert all diagnostics for a specific file.
///
/// Filters diagnostics to only those matching the given file_id.
pub fn convert_diagnostics_for_file(
    diagnostics: &[Diagnostic<usize>],
    file_id: usize,
    source: &str,
) -> Vec<LspDiagnostic> {
    diagnostics
        .iter()
        .filter(|d| {
            d.labels
                .first()
                .map(|l| l.file_id == file_id)
                .unwrap_or(false)
        })
        .filter_map(|d| convert_diagnostic(d, source))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_position_start() {
        let source = "hello\nworld";
        let pos = byte_offset_to_position(source, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_same_line() {
        let source = "hello\nworld";
        let pos = byte_offset_to_position(source, 3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }

    #[test]
    fn test_byte_offset_to_position_second_line() {
        let source = "hello\nworld";
        let pos = byte_offset_to_position(source, 6); // 'w' in "world"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_offset_to_position_middle_second_line() {
        let source = "hello\nworld";
        let pos = byte_offset_to_position(source, 8); // 'r' in "world"
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn test_byte_offset_beyond_end() {
        let source = "hello";
        let pos = byte_offset_to_position(source, 100);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }
}
