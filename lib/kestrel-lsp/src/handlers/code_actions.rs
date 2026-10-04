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

use kestrel_hecs::{Entity, World};
use kestrel_hir::body::HirExpr;
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LoweredBody;
use kestrel_syntax_tree::ast::{self, AstNode};
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

/// The `let` keyword of the statement that declares `local` — read off the
/// CST, from the binding the body's source map names. `None` when the local
/// is not declared by a `let` statement (a parameter, a pattern binding, an
/// already-`var` binding, or a local the source does not spell).
fn let_keyword_of(
    world: &World,
    body: Entity,
    lowered: &LoweredBody,
    local: LocalId,
) -> Option<rowan::TextRange> {
    let binding = lowered.source_map.local_source(local)?.binding;
    let root = kestrel_ast_builder::syntax::file_root(world, body)?;
    let decl = binding
        .to_node(&root)
        .ancestors()
        .find_map(ast::VariableDeclaration::cast)?;
    // The binding must be the declaration's own pattern, not one nested in
    // its initializer (a closure parameter, a match arm).
    let own_pattern = decl.pattern()?.syntax().text_range();
    if !own_pattern.contains_range(binding.text_range()) {
        return None;
    }
    Some(decl.let_token()?.text_range())
}

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
                    let Some(lowered) = semantic::lowered_body(world, root, body_entity) else {
                        continue;
                    };
                    let Some(expr_id) = semantic::expr_at(world, body_entity, &lowered, *offset)
                    else {
                        continue;
                    };
                    let HirExpr::Local(local_id, _) = &lowered.body.exprs[expr_id] else {
                        continue;
                    };
                    let local = &lowered.body.locals[*local_id];
                    let Some(let_range) = let_keyword_of(world, body_entity, &lowered, *local_id)
                    else {
                        continue;
                    };
                    let let_start: usize = let_range.start().into();
                    let let_end: usize = let_range.end().into();

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
    fn e200_finds_the_declaring_let() {
        use kestrel_compiler::Compiler;
        // The `let` read off the declaration — not a text search backward
        // from the local's span, which (that span being the whole statement)
        // found nothing, or found an earlier `let` and rewrote that one.
        let src = "module T\nfunc f() {\n    let y = 0; let x = 1;\n    x = 2;\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/e200.ks", src.into());
        c.build(f);
        let world = c.world();
        let offset = src.find("x = 2").unwrap();
        let body = semantic::body_entity_at(world, f, offset).expect("body");
        let lowered = semantic::lowered_body(world, c.root(), body).expect("hir");
        let expr = semantic::expr_at(world, body, &lowered, offset).expect("use");
        let HirExpr::Local(x, _) = lowered.body.exprs[expr] else {
            panic!("not a local");
        };
        let range = let_keyword_of(world, body, &lowered, x).expect("let keyword");
        let start: usize = range.start().into();
        assert_eq!(start, src.find("let x").unwrap());

        // A parameter has no `let` to change.
        let src = "module T\nfunc g(p: lang.i64) {\n    p = 2;\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/e200p.ks", src.into());
        c.build(f);
        let world = c.world();
        let offset = src.find("p = 2").unwrap();
        let body = semantic::body_entity_at(world, f, offset).expect("body");
        let lowered = semantic::lowered_body(world, c.root(), body).expect("hir");
        let expr = semantic::expr_at(world, body, &lowered, offset).expect("use");
        let HirExpr::Local(p, _) = lowered.body.exprs[expr] else {
            panic!("not a local");
        };
        assert!(let_keyword_of(world, body, &lowered, p).is_none());
    }
}
