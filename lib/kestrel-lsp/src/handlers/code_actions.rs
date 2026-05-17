//! `textDocument/codeAction` — quick-fixes for analyzer diagnostics.
//!
//! For each `lsp_types::Diagnostic` in the request context whose `code`
//! matches a known descriptor ID, build a `CodeAction` with a `WorkspaceEdit`
//! that performs the fix. The descriptor ID lands in `Diagnostic.code` via
//! `convert.rs::AnalyzeDiagnostic→Diagnostic`.
//!
//! First implemented fix:
//! - **E002 — unreachable_code**: delete the unreachable statement /
//!   expression. The diagnostic's primary range already spans exactly the
//!   code to remove; we extend it forward to consume the trailing newline
//!   so the file doesn't keep a blank line.

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Diagnostic, NumberOrString, Position, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::server::{SharedState, url_to_path};

pub async fn handle(state: SharedState, params: CodeActionParams) -> Option<CodeActionResponse> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let source = {
        let s = state.lock().await;
        s.sources.get(&path).cloned()
    };

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();
    for diag in &params.context.diagnostics {
        if let Some(action) = build_action(diag, &uri, source.as_deref()) {
            actions.push(CodeActionOrCommand::CodeAction(action));
        }
    }
    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn build_action(diag: &Diagnostic, uri: &Url, source: Option<&str>) -> Option<CodeAction> {
    let code = match diag.code.as_ref()? {
        NumberOrString::String(s) => s.as_str(),
        NumberOrString::Number(_) => return None,
    };
    match code {
        "E002" => Some(remove_dead_code_action(diag, uri, source)),
        _ => None,
    }
}

/// E002 fix: delete the unreachable statement/expression range. We extend
/// the deletion forward to swallow the trailing newline so we don't leave
/// a blank line behind.
fn remove_dead_code_action(diag: &Diagnostic, uri: &Url, source: Option<&str>) -> CodeAction {
    let extended_end = source
        .map(|src| extend_through_newline(src, diag.range.end))
        .unwrap_or(diag.range.end);
    let edit_range = Range {
        start: diag.range.start,
        end: extended_end,
    };

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: edit_range,
            new_text: String::new(),
        }],
    );

    CodeAction {
        title: "Remove unreachable code".into(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    }
}

/// If the character at `pos` is the start of a line break, advance past it
/// (handles `\n` and `\r\n`). Otherwise return `pos` unchanged.
fn extend_through_newline(source: &str, pos: Position) -> Position {
    let mut line: usize = 0;
    let mut col_utf16: usize = 0;
    let target_line = pos.line as usize;
    let target_col = pos.character as usize;
    let mut chars = source.char_indices().peekable();
    while let Some(&(_, c)) = chars.peek() {
        if line == target_line && col_utf16 == target_col {
            // Found the position; check if next char is a newline.
            return match c {
                '\n' => Position {
                    line: pos.line + 1,
                    character: 0,
                },
                '\r' => {
                    let _ = chars.next();
                    if matches!(chars.peek(), Some(&(_, '\n'))) {
                        Position {
                            line: pos.line + 1,
                            character: 0,
                        }
                    } else {
                        pos
                    }
                },
                _ => pos,
            };
        }
        let _ = chars.next();
        if c == '\n' {
            line += 1;
            col_utf16 = 0;
        } else {
            col_utf16 += c.len_utf16();
        }
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_through_newline_swallows_lf() {
        let src = "abc\ndef\n";
        let pos = Position {
            line: 0,
            character: 3,
        };
        let extended = extend_through_newline(src, pos);
        assert_eq!(extended.line, 1);
        assert_eq!(extended.character, 0);
    }

    #[test]
    fn extend_through_newline_swallows_crlf() {
        let src = "abc\r\ndef";
        let pos = Position {
            line: 0,
            character: 3,
        };
        let extended = extend_through_newline(src, pos);
        assert_eq!(extended.line, 1);
        assert_eq!(extended.character, 0);
    }

    #[test]
    fn extend_through_newline_noop_when_not_eol() {
        let src = "abcdef";
        let pos = Position {
            line: 0,
            character: 3,
        };
        let extended = extend_through_newline(src, pos);
        assert_eq!(extended, pos);
    }

    #[test]
    fn build_action_e002_returns_quickfix() {
        let uri = Url::parse("file:///tmp/x.ks").unwrap();
        let diag = Diagnostic {
            range: Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 10,
                },
            },
            code: Some(NumberOrString::String("E002".into())),
            message: "unreachable code".into(),
            ..Default::default()
        };
        let action = build_action(&diag, &uri, Some("ok\nok\nbad code\nrest")).expect("action");
        assert_eq!(action.kind, Some(CodeActionKind::QUICKFIX));
        assert!(action.title.contains("Remove unreachable"));
        let edit = action.edit.unwrap();
        let changes = edit.changes.unwrap();
        let edits = &changes[&uri];
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "");
    }

    #[test]
    fn build_action_unknown_code_returns_none() {
        let uri = Url::parse("file:///tmp/x.ks").unwrap();
        let diag = Diagnostic {
            code: Some(NumberOrString::String("E999".into())),
            ..Default::default()
        };
        assert!(build_action(&diag, &uri, None).is_none());
    }
}
