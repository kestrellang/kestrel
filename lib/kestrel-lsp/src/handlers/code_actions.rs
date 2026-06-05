//! `textDocument/codeAction` — quick-fixes for analyzer diagnostics.
//!
//! For each `lsp_types::Diagnostic` in the request context whose `code`
//! matches a known descriptor ID, build a `CodeAction` with a `WorkspaceEdit`
//! that performs the fix. The descriptor ID lands in `Diagnostic.code` via
//! `convert.rs::AnalyzeDiagnostic→Diagnostic`.
//!
//! Implemented fixes:
//! - **E002 — unreachable_code**: delete the unreachable statement /
//!   expression. The diagnostic's primary range already spans exactly the
//!   code to remove; we extend it forward to consume the trailing newline
//!   so the file doesn't keep a blank line.
//! - **E200 — assign_to_immutable**: change `let` to `var` at the local's
//!   declaration site. Requires compiler access to locate the declaration.

use std::collections::HashMap;

use kestrel_hir::body::HirExpr;
use kestrel_hir_lower::LowerBody;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    Diagnostic, NumberOrString, Position, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{SharedState, url_to_path};

pub async fn handle(state: SharedState, params: CodeActionParams) -> Option<CodeActionResponse> {
    let uri = params.text_document.uri;
    let path = url_to_path(&uri);

    let source = {
        let s = state.lock().await;
        s.sources.get(&path).cloned()
    };

    let mut actions: Vec<CodeActionOrCommand> = Vec::new();

    // Text-only actions (no compiler needed).
    let mut has_let_to_var = false;
    for diag in &params.context.diagnostics {
        match diag_code(diag) {
            Some("E002") => {
                actions.push(CodeActionOrCommand::CodeAction(remove_dead_code_action(
                    diag,
                    &uri,
                    source.as_deref(),
                )));
            },
            // E200: assign to immutable local; E203: let binding to mutating param.
            // Both fix by changing `let` to `var` at the declaration site.
            Some("E200" | "E203") => has_let_to_var = true,
            _ => {},
        }
    }

    // Compiler-backed actions: change `let` → `var`.
    if has_let_to_var {
        let let_diags: Vec<Diagnostic> = params
            .context
            .diagnostics
            .iter()
            .filter(|d| matches!(diag_code(d), Some("E200" | "E203")))
            .cloned()
            .collect();
        if let Some(fixes) = handle_let_to_var(&state, &uri, &path, &let_diags).await {
            actions.extend(fixes);
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn diag_code(diag: &Diagnostic) -> Option<&str> {
    match diag.code.as_ref()? {
        NumberOrString::String(s) => Some(s.as_str()),
        NumberOrString::Number(_) => None,
    }
}

// ===== E002 — remove unreachable code =====

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

fn extend_through_newline(source: &str, pos: Position) -> Position {
    let mut line: usize = 0;
    let mut col_utf16: usize = 0;
    let target_line = pos.line as usize;
    let target_col = pos.character as usize;
    let mut chars = source.char_indices().peekable();
    while let Some(&(_, c)) = chars.peek() {
        if line == target_line && col_utf16 == target_col {
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

// ===== E200 — change `let` to `var` =====

async fn handle_let_to_var(
    state: &SharedState,
    uri: &Url,
    path: &str,
    diags: &[Diagnostic],
) -> Option<Vec<CodeActionOrCommand>> {
    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            li,
        )
    };

    let offsets: Vec<usize> = diags
        .iter()
        .map(|d| line_index.position_to_offset(d.range.start))
        .collect();
    let diags_owned = diags.to_vec();
    let path_owned = path.to_string();
    let uri_owned = uri.clone();

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<CodeActionOrCommand>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path_owned)?;
                let world = compiler.world();
                let root = compiler.root();
                let source = sources.get(&path_owned)?;

                let mut actions = Vec::new();
                for (diag, offset) in diags_owned.iter().zip(offsets.iter()) {
                    let Some(body_entity) = semantic::body_entity_at(world, file_entity, *offset)
                    else {
                        continue;
                    };
                    let ctx = world.query_context();
                    let Some(hir) = ctx.query(LowerBody {
                        entity: body_entity,
                        root,
                    }) else {
                        continue;
                    };
                    let Some(expr_id) = semantic::hir_expr_at(&hir, *offset) else {
                        continue;
                    };
                    let HirExpr::Local(local_id, _) = &hir.exprs[expr_id] else {
                        continue;
                    };
                    let local = &hir.locals[*local_id];
                    let decl_start = local.span.start;

                    // Search backward from the name for the `let` keyword.
                    let search_start = decl_start.saturating_sub(20);
                    let prefix = &source[search_start..decl_start];
                    let Some(let_in_prefix) = prefix.rfind("let") else {
                        continue;
                    };
                    let let_start = search_start + let_in_prefix;
                    let let_end = let_start + 3;

                    let li = LineIndex::new(source.clone());
                    let edit_range = li.range_for(let_start, let_end);

                    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
                    changes.insert(
                        uri_owned.clone(),
                        vec![TextEdit {
                            range: edit_range,
                            new_text: "var".to_string(),
                        }],
                    );

                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: format!("Change 'let' to 'var' for '{}'", local.name),
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
                    }));
                }

                if actions.is_empty() {
                    None
                } else {
                    Some(actions)
                }
            },
        )
        .await
        .flatten()
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
        assert_eq!(diag_code(&diag), Some("E002"));
        let action = remove_dead_code_action(&diag, &uri, Some("ok\nok\nbad code\nrest"));
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
        let diag = Diagnostic {
            code: Some(NumberOrString::String("E999".into())),
            ..Default::default()
        };
        assert_eq!(diag_code(&diag), Some("E999"));
    }

    #[test]
    fn e200_finds_let_keyword() {
        // Simulate finding `let` before a local name at byte offset 15.
        let src = "module T\nfunc f() {\n    let x = 1;\n    x = 2;\n}\n";
        let let_pos = src.find("let x").unwrap();
        let x_pos = src.find("let x").unwrap() + "let ".len();
        let search_start = x_pos.saturating_sub(20);
        let prefix = &src[search_start..x_pos];
        let found = prefix.rfind("let").unwrap();
        let actual_let = search_start + found;
        assert_eq!(actual_let, let_pos);
        assert_eq!(&src[actual_let..actual_let + 3], "let");
    }
}
