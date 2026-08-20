//! `textDocument/prepareRename` and `textDocument/rename`.
//!
//! Both methods share the same dispatch as `references.rs`: locate the
//! entity (or local) at the cursor. `prepareRename` returns the identifier
//! range + current text as the popup placeholder. `rename` validates the new
//! name with the lexer, runs `references_to` for use sites, performs a
//! per-site collision check via `ResolveName`, and bundles the resulting
//! `TextEdit`s into a `WorkspaceEdit`.

use std::collections::HashMap;

use kestrel_ast_builder::{CstNode, DeclSpan, FilePath, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_lexer::{Token, lex};
use kestrel_name_res::{NameResolution, ResolveName};
use kestrel_span::Span;
use kestrel_syntax_tree::utils::get_name_span;
use kestrel_type_infer::InferBody;
use tower_lsp::jsonrpc::{Error as RpcError, ErrorCode};
use tower_lsp::lsp_types::{
    PrepareRenameResponse, RenameParams, TextDocumentPositionParams, TextEdit, Url, WorkspaceEdit,
};

use crate::position::LineIndex;
use crate::references::{
    self, RefKind, ReferenceSite, clip_to_identifier, decl_at_name_offset, span_spells_name,
};
use crate::semantic;
use crate::server::{SharedState, path_to_url, url_to_path};

// ===== prepareRename =====

pub async fn prepare(
    state: SharedState,
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>, RpcError> {
    let uri = params.text_document.uri;
    let pos = params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let Some(li) = s.docs.get(&uri).map(|d| d.line_index.clone()) else {
            return Ok(None);
        };
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            li,
        )
    };
    let offset = line_index.position_to_offset(pos);

    let result = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<PrepareRenameResponse> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                let target = target_at(world, file_entity, offset, root)?;
                let (placeholder, span) = identifier_for_target(world, root, &target, &sources)?;
                let li = LineIndex::new(sources.get(&path)?.clone());
                let range = li.range_for(span.start, span.end);

                Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder })
            },
        )
        .await
        .flatten();

    Ok(result)
}

// ===== rename =====

pub async fn rename(
    state: SharedState,
    params: RenameParams,
) -> Result<Option<WorkspaceEdit>, RpcError> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let new_name = params.new_name.clone();
    let path = url_to_path(&uri);

    if let Err(msg) = validate_identifier(&new_name) {
        return Err(RpcError {
            code: ErrorCode::InvalidParams,
            message: msg.into(),
            data: None,
        });
    }

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let Some(li) = s.docs.get(&uri).map(|d| d.line_index.clone()) else {
            return Ok(None);
        };
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            li,
        )
    };
    let offset = line_index.position_to_offset(pos);

    let outcome: Result<Option<WorkspaceEdit>, RpcError> = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Result<Option<WorkspaceEdit>, RpcError> {
                let file_entity = match semantic::file_entity_for_path(compiler, &path) {
                    Some(f) => f,
                    None => return Ok(None),
                };
                let world = compiler.world();
                let root = compiler.root();

                let target = match target_at(world, file_entity, offset, root) {
                    Some(t) => t,
                    None => return Ok(None),
                };

                // Stdlib / overload-set / not-an-identifier-span guard.
                // identifier_for_target returns None for all of them, so
                // prepareRename already filters most of these — but a client
                // can call rename without prepareRename, so re-check.
                if identifier_for_target(world, root, &target, &sources).is_none() {
                    return Err(RpcError {
                        code: ErrorCode::InvalidRequest,
                        message: "this symbol cannot be renamed".into(),
                        data: None,
                    });
                }

                let mut sites = collect_sites(world, root, &target);

                // Add the declaration site itself so its text changes too.
                push_decl_site(world, root, &target, &sources, &mut sites);

                check_collisions(world, root, &target, &new_name, &sites)?;

                let edit = build_workspace_edit(world, &sources, &sites, &new_name);
                Ok(Some(edit))
            },
        )
        .await
        .unwrap_or_else(|| Err(RpcError::internal_error()));

    outcome
}

// ===== Shared dispatch =====

/// What the cursor resolves to (mirrors `references.rs::Target`).
enum Target {
    Entity(Entity),
    Local { body: Entity, id: LocalId },
}

fn target_at(world: &World, file_entity: Entity, offset: usize, root: Entity) -> Option<Target> {
    if let Some(body_entity) = semantic::body_entity_at(world, file_entity, offset) {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) && let Some(expr_id) = semantic::hir_expr_at(&hir, offset)
            && let Some(t) = resolve_expr(&hir, body_entity, expr_id, &ctx, root)
        {
            return Some(t);
        }
    }
    // Fallback: the cursor is on a declaration's own identifier (function
    // name, struct field, …). `decl_at_name_offset` — not bare
    // `enclosing_decl_at` — because the latter maps *every* offset inside a
    // decl to that decl, so a cursor on a `let` binding or a parameter name
    // (neither is an `HirExpr`, so the branch above can't see them) would
    // resolve to the enclosing function and rename it workspace-wide.
    let decl = decl_at_name_offset(world, file_entity, offset)?;
    Some(Target::Entity(decl))
}

fn resolve_expr(
    hir: &HirBody,
    body: Entity,
    expr_id: HirExprId,
    ctx: &kestrel_hecs::QueryContext<'_>,
    root: Entity,
) -> Option<Target> {
    match &hir.exprs[expr_id] {
        HirExpr::Def(entity, _, _) => Some(Target::Entity(*entity)),
        HirExpr::Local(local_id, _) => Some(Target::Local {
            body,
            id: *local_id,
        }),
        HirExpr::OverloadSet { .. } => None, // ambiguous — disqualify
        HirExpr::MethodCall { .. }
        | HirExpr::Field { .. }
        | HirExpr::Call { .. }
        | HirExpr::ImplicitMember { .. }
        | HirExpr::ProtocolCall { .. } => {
            let typed = ctx.query(InferBody { entity: body, root })?;
            typed.resolutions.get(&expr_id).copied().map(Target::Entity)
        },
        _ => None,
    }
}

/// Source text of the file that owns `entity`, keyed the way the server keys
/// `sources` (canonical `FilePath`). `None` for stdlib entities (no `FilePath`
/// ancestor) and for files the server has no text for.
fn source_of<'a>(
    world: &World,
    sources: &'a HashMap<String, String>,
    entity: Entity,
) -> Option<&'a str> {
    let file = crate::references::entity_file(world, entity)?;
    let path = world.get::<FilePath>(file).map(|p| p.0.clone())?;
    sources.get(&path).map(|s| s.as_str())
}

/// Get the identifier text + span we'd rename for a target. Returns `None`
/// for targets we refuse to rename: stdlib entities (no source span we can
/// edit), overload sets (would need to fix every overload), and — the guard
/// that makes this fail *closed* — any target whose span does not literally
/// spell its own name.
///
/// That last check is what stops rename from corrupting source. A local's
/// `Local::span` is not the binding's identifier: for `let count = …` it is
/// the whole statement, for a parameter or `self` it is `Span::synthetic(0)`
/// (`0..0`, i.e. the top of some *other* file), and for every desugaring temp
/// it is the span of the construct that produced it. Editing any of those
/// destroys the statement or splices text at offset 0. Until `Local` carries a
/// real `name_span` (Stage 2/3 of F2) the honest answer for those targets is
/// "this symbol cannot be renamed".
///
/// `Target::Entity` is checked too, as defence in depth: `get_name_span`
/// should always yield the identifier there, and if it ever doesn't, that is a
/// real bug and refusing to edit is the right response.
fn identifier_for_target(
    world: &World,
    root: Entity,
    target: &Target,
    sources: &HashMap<String, String>,
) -> Option<(String, Span)> {
    let (owner, name, span) = match target {
        Target::Entity(e) => {
            // Reject stdlib entities — they have no FilePath ancestor.
            crate::references::entity_file(world, *e)?;
            // Reject if no `Name` (anonymous decl).
            let name = world.get::<Name>(*e).map(|n| n.0.clone())?;
            // Reject modules — `module` declaration spans the whole file in
            // some grammars and renaming it changes the file name semantics.
            if matches!(world.get::<NodeKind>(*e), Some(&NodeKind::Module)) {
                return None;
            }
            let cst = world.get::<CstNode>(*e)?;
            let decl_span = world.get::<DeclSpan>(*e)?;
            let span = get_name_span(&cst.0, decl_span.0.file_id)?;
            (*e, name, span)
        },
        Target::Local { body, id } => {
            let ctx = world.query_context();
            let hir = ctx.query(LowerBody {
                entity: *body,
                root,
            })?;
            let local = &hir.locals[*id];
            (*body, local.name.clone(), local.span.clone())
        },
    };

    // Fail closed: only rename a span that already reads as the name.
    let source = source_of(world, sources, owner)?;
    if !span_spells_name(source, &span, &name) {
        return None;
    }
    Some((name, span))
}

fn collect_sites(world: &World, root: Entity, target: &Target) -> Vec<ReferenceSite> {
    match target {
        Target::Entity(e) => references::references_to(world, root, *e),
        Target::Local { body, id } => references::local_references(world, *body, root, *id),
    }
}

fn push_decl_site(
    world: &World,
    root: Entity,
    target: &Target,
    sources: &HashMap<String, String>,
    sites: &mut Vec<ReferenceSite>,
) {
    if let Some((_, span)) = identifier_for_target(world, root, target, sources) {
        let file = match target {
            Target::Entity(e) => crate::references::entity_file(world, *e),
            Target::Local { body, .. } => crate::references::entity_file(world, *body),
        };
        if let Some(file) = file {
            sites.push(ReferenceSite {
                file,
                span,
                kind: RefKind::Direct,
            });
        }
    }
}

// ===== Validation =====

fn validate_identifier(s: &str) -> Result<(), &'static str> {
    if s.is_empty() {
        return Err("rename target cannot be empty");
    }
    let mut tokens = lex(s, 0).filter(|t| match t {
        Ok(spanned) => !spanned.value.is_trivia(),
        Err(_) => true,
    });
    let first = tokens.next();
    let extra = tokens.next();
    match (first, extra) {
        (Some(Ok(spanned)), None) if matches!(spanned.value, Token::Identifier) => Ok(()),
        (Some(Ok(_)), None) => Err("not a valid identifier (keyword or symbol)"),
        _ => Err("not a valid identifier"),
    }
}

fn check_collisions(
    world: &World,
    root: Entity,
    target: &Target,
    new_name: &str,
    sites: &[ReferenceSite],
) -> Result<(), RpcError> {
    // For locals, only check intra-body collisions (other locals with same
    // name in the body). Workspace-level shadowing of a free name is left to
    // the user — local rename is opt-in, not auto-shadow-detection.
    if let Target::Local { body, id } = target {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: *body,
            root,
        }) {
            for (lid, local) in hir.locals.iter() {
                if lid != *id && local.name == new_name {
                    return Err(RpcError {
                        code: ErrorCode::InvalidRequest,
                        message: format!("`{new_name}` is already used by another local").into(),
                        data: None,
                    });
                }
            }
        }
        return Ok(());
    }

    // For entity targets, consult `ResolveName` from the scope at each use
    // site. If it resolves to anything other than the target, that's a
    // collision.
    let target_entity = match target {
        Target::Entity(e) => *e,
        _ => unreachable!(),
    };

    let ctx = world.query_context();
    for site in sites {
        // Find the smallest enclosing decl at the site for scope context.
        let context = match semantic::enclosing_decl_at(world, site.file, site.span.start) {
            Some(c) => c,
            None => continue,
        };
        let res = ctx.query(ResolveName {
            name: new_name.to_string(),
            context,
            root,
        });
        if would_collide(&res, target_entity) {
            return Err(RpcError {
                code: ErrorCode::InvalidRequest,
                message: format!(
                    "`{new_name}` already resolves to a different symbol at one of the use sites"
                )
                .into(),
                data: None,
            });
        }
    }
    Ok(())
}

fn would_collide(res: &NameResolution, target: Entity) -> bool {
    match res {
        NameResolution::Found(entities) => {
            // Collision only if the resolution doesn't already include our
            // target. Function overloads share names so seeing the target
            // in the list is fine.
            !entities.contains(&target)
        },
        NameResolution::Ambiguous(_) => true,
        NameResolution::NotFound => false,
    }
}

// ===== Edit assembly =====

fn build_workspace_edit(
    world: &World,
    sources: &HashMap<String, String>,
    sites: &[ReferenceSite],
    new_name: &str,
) -> WorkspaceEdit {
    let mut by_url: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    let mut indices: HashMap<Entity, LineIndex> = HashMap::new();

    for site in sites {
        let Some(file_path) = world.get::<FilePath>(site.file).map(|p| p.0.clone()) else {
            continue;
        };
        let Some(url) = path_to_url(&file_path) else {
            continue;
        };
        let Some(source) = sources.get(&file_path) else {
            continue;
        };
        let li = indices
            .entry(site.file)
            .or_insert_with(|| LineIndex::new(source.clone()));

        let clipped = clip_to_identifier(source, &site.span, site.kind);
        let range = li.range_for(clipped.start, clipped.end);
        by_url.entry(url).or_default().push(TextEdit {
            range,
            new_text: new_name.to_string(),
        });
    }

    // Dedupe identical edits within a file (decl span is added by
    // push_decl_site even when references_to already returned it).
    for edits in by_url.values_mut() {
        edits.sort_by_key(|e| (e.range.start.line, e.range.start.character));
        edits.dedup_by(|a, b| a.range == b.range);
    }

    WorkspaceEdit {
        changes: Some(by_url),
        document_changes: None,
        change_annotations: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::FileId;
    use kestrel_compiler::Compiler;

    fn find_decl(world: &World, file: Entity, name: &str) -> Entity {
        for (e, n) in world.iter_component::<Name>() {
            if n.0 != name {
                continue;
            }
            if let Some(fid) = world.get::<FileId>(e)
                && fid.0 == file
            {
                return e;
            }
        }
        panic!("no decl `{name}` in file");
    }

    /// Find a local by name inside `body`'s lowered HIR. Compiler-generated
    /// locals (`$iter`, `$let_tmp`, `_cparam_0`, …) have no clickable source
    /// text, so tests that need them address them by name through here rather
    /// than through a cursor offset.
    fn find_local(world: &World, body: Entity, root: Entity, name: &str) -> LocalId {
        let ctx = world.query_context();
        let hir = ctx
            .query(LowerBody { entity: body, root })
            .expect("body lowers");
        hir.locals
            .iter()
            .find(|(_, l)| l.name == name)
            .map(|(id, _)| id)
            .unwrap_or_else(|| {
                let have: Vec<&str> = hir.locals.iter().map(|(_, l)| l.name.as_str()).collect();
                panic!("no local `{name}`; body has {have:?}")
            })
    }

    /// Build the `sources` map the handlers take, for a single-file fixture.
    fn sources_of(path: &str, src: &str) -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert(path.to_string(), src.to_string());
        m
    }

    /// Fixture shared by the Stage-1 refusal tests. One function, one
    /// parameter, one `let` local, two use sites of each.
    const LOCALS_SRC: &str = "module Test\n\
                              func foo(bar: lang.i64) -> lang.i64 { let count = bar; count + bar }\n";

    // ===== F2 Stage 1: renames that would corrupt source must be refused =====
    //
    // Every test below asserts a *refusal*. That is the Stage-1 target, not
    // the intended end state: `Local::span` is not the binding's identifier
    // (it is the whole `let` statement, or `Span::synthetic(0)` for a
    // parameter), so any edit derived from it damages the file. Refusing is
    // strictly better than corrupting. Stage 2 (`Local::name_span`), Stage 3
    // (`AstParam::name_span`) and Stage 4 (`semantic::local_decl_at`) will
    // make these renames actually work — at which point these assertions
    // should be *flipped* to assert the correct edits, not deleted.

    #[test]
    fn local_let_rename_is_refused() {
        // Renaming a `let` local must be refused. `Local::span` for a simple
        // binding is the whole statement (`hir-lower/src/stmt.rs:126`), so
        // before the guard this produced a TextEdit replacing
        // `let count = bar;` with `renamed`.
        //
        // Stage 2 will flip this to assert an edit covering just `count`.
        let mut c = Compiler::new();
        let path = "/tmp/rename_local_let.ks";
        let f = c.set_source(path, LOCALS_SRC.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());
        let sources = sources_of(path, LOCALS_SRC);

        let offset = LOCALS_SRC.rfind("count").expect("use site") + 1;
        let target = target_at(world, f, offset, root).expect("cursor resolves to the local");
        assert!(
            matches!(target, Target::Local { .. }),
            "cursor on a `count` use should resolve to a local"
        );

        // Root cause, pinned: the span we would have edited is the statement.
        let foo = find_decl(world, f, "foo");
        let id = find_local(world, foo, root, "count");
        let ctx = world.query_context();
        let hir = ctx.query(LowerBody { entity: foo, root }).expect("hir");
        let span = &hir.locals[id].span;
        assert_eq!(
            &LOCALS_SRC[span.start..span.end],
            "let count = bar;",
            "Local::span is the statement, not the identifier"
        );

        // The harm, asserted first: no edit may rewrite the statement.
        let mut sites = collect_sites(world, root, &target);
        push_decl_site(world, root, &target, &sources, &mut sites);
        let edit = build_workspace_edit(world, &sources, &sites, "renamed");
        let li = LineIndex::new(LOCALS_SRC.to_string());
        let stmt = li.range_for(span.start, span.end);
        for edits in edit.changes.iter().flat_map(|c| c.values()) {
            assert!(
                !edits.iter().any(|e| e.range == stmt),
                "an edit replaces the whole `let count = bar;` statement: {edits:?}"
            );
        }

        assert!(
            identifier_for_target(world, root, &target, &sources).is_none(),
            "renaming a `let` local must be refused while Local::span is the statement"
        );
    }

    #[test]
    fn parameter_rename_is_refused() {
        // Renaming a parameter must be refused. Parameter locals are created
        // with `Span::synthetic(0)` (`hir-lower/src/lib.rs:79`), i.e.
        // `{file_id: 0, start: 0, end: 0}` — before the guard this emitted an
        // insertion at line 0, character 0 of whichever file has id 0,
        // prepending `renamed` to the top of the file.
        //
        // Stage 3 (`AstParam::name_span`) will flip this to assert a real edit.
        let mut c = Compiler::new();
        let path = "/tmp/rename_param.ks";
        let f = c.set_source(path, LOCALS_SRC.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());
        let sources = sources_of(path, LOCALS_SRC);

        let offset = LOCALS_SRC.rfind("bar").expect("use site") + 1;
        let target = target_at(world, f, offset, root).expect("cursor resolves to the param");
        assert!(matches!(target, Target::Local { .. }));

        // The harm, asserted first: no edit at (0,0)-(0,0).
        let mut sites = collect_sites(world, root, &target);
        push_decl_site(world, root, &target, &sources, &mut sites);
        let edit = build_workspace_edit(world, &sources, &sites, "renamed");
        let origin = LineIndex::new(LOCALS_SRC.to_string()).range_for(0, 0);
        for edits in edit.changes.iter().flat_map(|c| c.values()) {
            assert!(
                !edits.iter().any(|e| e.range == origin),
                "an edit splices text at offset 0 of the file: {edits:?}"
            );
        }

        assert!(
            identifier_for_target(world, root, &target, &sources).is_none(),
            "renaming a parameter must be refused while its span is synthetic"
        );
    }

    #[test]
    fn self_rename_is_refused() {
        // `self` is the permanent case: refusal here is correct forever, not a
        // Stage-1 stopgap. `ReceiverKind` (`ast-builder/src/components.rs`) is
        // derived from the presence of `mutating` / `consuming` keywords —
        // there is no `self:` token in a Kestrel signature to point a span at,
        // so `self` can never acquire a name span to rewrite.
        let src = "module Test\n\
                   struct S { var field: lang.i64; }\n\
                   extend S { func read() -> lang.i64 { self.field } }\n";
        let mut c = Compiler::new();
        let path = "/tmp/rename_self.ks";
        let f = c.set_source(path, src.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());
        let sources = sources_of(path, src);

        let offset = src.find("self.field").expect("receiver use") + 1;
        let target = target_at(world, f, offset, root).expect("cursor resolves");
        assert!(
            matches!(target, Target::Local { .. }),
            "cursor on `self` should resolve to the `self` local"
        );
        assert!(
            identifier_for_target(world, root, &target, &sources).is_none(),
            "`self` can never be renamed"
        );
    }

    #[test]
    fn desugared_local_rename_is_refused() {
        // Desugaring temps (`$iter`, `$try_value`, `$dsi`, `$opts`,
        // `$let_tmp`, `_cparam_N`) have no source text at all, but do carry a
        // real, non-synthetic span — the span of the construct that produced
        // them. A client can't click one, but it can send a rename whose
        // offset resolves to one, and `Target::Local` is reachable from other
        // paths, so the refusal is checked directly.
        //
        // This one stays a refusal after Stage 2/3: there is nothing in the
        // file to rewrite.
        //
        // The temp used here is `$let_tmp` (destructuring `let`) rather than
        // `$iter`: `for … in` desugaring needs the `Iterable` builtin, and
        // `Compiler::new()` in these unit tests has no stdlib, so a `for` loop
        // short-circuits to `HirExpr::Error` and never defines `$iter`. Both
        // temps take the enclosing construct's span, so the guard sees the
        // same shape.
        let src = "module Test\n\
                   func pair_sum() -> lang.i64 {\n  \
                     let (a, b) = (1, 2);\n  \
                     a + b\n\
                   }\n";
        let mut c = Compiler::new();
        let path = "/tmp/rename_desugar.ks";
        let f = c.set_source(path, src.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());
        let sources = sources_of(path, src);

        let body = find_decl(world, f, "pair_sum");
        let id = find_local(world, body, root, "$let_tmp");
        let target = Target::Local { body, id };
        assert!(
            identifier_for_target(world, root, &target, &sources).is_none(),
            "a desugaring temp has no identifier in the source and must be refused"
        );
    }

    #[test]
    fn rename_from_let_pattern_declaration_is_refused() {
        // The most natural rename gesture: cursor on the binding in
        // `let count = bar;` itself. A binding's own identifier is not an
        // `HirExpr`, so `hir_expr_at` misses it and `target_at` used to fall
        // through to `enclosing_decl_at`, which resolves *any* offset inside
        // `foo` to `foo` — renaming the enclosing function workspace-wide.
        //
        // Stage 4 (`semantic::local_decl_at`) will flip this to resolve to
        // `Target::Local`.
        let mut c = Compiler::new();
        let path = "/tmp/rename_let_decl.ks";
        let f = c.set_source(path, LOCALS_SRC.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());

        let offset = LOCALS_SRC.find("count").expect("binding site") + 1;
        let target = target_at(world, f, offset, root);
        let resolved = target.as_ref().map(|t| match t {
            Target::Entity(e) => world
                .get::<Name>(*e)
                .map(|n| n.0.clone())
                .unwrap_or_default(),
            Target::Local { .. } => "<local>".to_string(),
        });
        assert!(
            target.is_none(),
            "cursor on the `count` binding must not resolve to anything renameable, got {resolved:?}"
        );
    }

    #[test]
    fn rename_from_parameter_declaration_is_refused() {
        // Same defect at the parameter declaration: cursor on `bar` in the
        // signature. The offset is outside the body, so `hir_expr_at` never
        // runs and `enclosing_decl_at` returned `foo`.
        let mut c = Compiler::new();
        let path = "/tmp/rename_param_decl.ks";
        let f = c.set_source(path, LOCALS_SRC.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());

        let offset = LOCALS_SRC.find("bar").expect("param declaration") + 1;
        let target = target_at(world, f, offset, root);
        let resolved = target.as_ref().map(|t| match t {
            Target::Entity(e) => world
                .get::<Name>(*e)
                .map(|n| n.0.clone())
                .unwrap_or_default(),
            Target::Local { .. } => "<local>".to_string(),
        });
        assert!(
            target.is_none(),
            "cursor on the `bar` parameter must not resolve to the enclosing function, got {resolved:?}"
        );
    }

    #[test]
    fn rename_from_type_reference_in_body_is_refused() {
        // Confirmed while implementing Stage 1: `rename::target_at` has no
        // `type_at_cursor` pre-check (unlike find-references and
        // document-highlight), so a cursor on `Foo` in `let x: Foo = …` hit
        // the same `enclosing_decl_at` fallback and renamed the *enclosing
        // function*. Guard B fixes it for free.
        //
        // Making this rename `Foo` is a separate follow-up: teach
        // `rename::target_at` the `type_at_cursor` branch the other two
        // handlers already have.
        let src = "module Test\n\
                   struct Foo { var a: lang.i64; }\n\
                   func use_it() -> lang.i64 { let x: Foo = Foo(a: 1); x.a }\n";
        let mut c = Compiler::new();
        let path = "/tmp/rename_type_ref.ks";
        let f = c.set_source(path, src.into());
        c.build(f);
        let (world, root) = (c.world(), c.root());

        let offset = src.find(": Foo").expect("type annotation") + 2;
        let target = target_at(world, f, offset, root);
        let resolved = target.as_ref().map(|t| match t {
            Target::Entity(e) => world
                .get::<Name>(*e)
                .map(|n| n.0.clone())
                .unwrap_or_default(),
            Target::Local { .. } => "<local>".to_string(),
        });
        assert_ne!(
            resolved.as_deref(),
            Some("use_it"),
            "cursor on a type annotation must not resolve to the enclosing function"
        );
    }

    #[test]
    fn validate_accepts_identifier() {
        assert!(validate_identifier("foo").is_ok());
        assert!(validate_identifier("_bar").is_ok());
        assert!(validate_identifier("snake_case").is_ok());
    }

    #[test]
    fn validate_rejects_keyword() {
        assert!(validate_identifier("if").is_err());
        assert!(validate_identifier("func").is_err());
        assert!(validate_identifier("struct").is_err());
    }

    #[test]
    fn validate_rejects_empty_or_invalid() {
        assert!(validate_identifier("").is_err());
        assert!(validate_identifier("123").is_err());
        assert!(validate_identifier("foo bar").is_err());
        assert!(validate_identifier("foo+").is_err());
    }

    #[test]
    fn collision_check_rejects_existing_name() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func foo() -> lang.i64 { 1 }\n\
                   func bar() -> lang.i64 { foo() }\n";
        let f = c.set_source("/tmp/rename_collision.ks", src.into());
        c.build(f);

        let foo = find_decl(c.world(), f, "foo");
        let target = Target::Entity(foo);
        let sites = collect_sites(c.world(), c.root(), &target);

        // Renaming `foo` → `bar` should collide because `bar` already exists.
        let result = check_collisions(c.world(), c.root(), &target, "bar", &sites);
        assert!(result.is_err(), "expected collision, got {result:?}");
    }

    #[test]
    fn collision_check_allows_unused_name() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func foo() -> lang.i64 { 1 }\n\
                   func bar() -> lang.i64 { foo() }\n";
        let f = c.set_source("/tmp/rename_ok.ks", src.into());
        c.build(f);

        let foo = find_decl(c.world(), f, "foo");
        let target = Target::Entity(foo);
        let sites = collect_sites(c.world(), c.root(), &target);

        let result = check_collisions(c.world(), c.root(), &target, "fresh_unused_name", &sites);
        assert!(result.is_ok(), "expected no collision, got {result:?}");
    }

    #[test]
    fn workspace_edit_includes_call_site_and_decl() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func foo() -> lang.i64 { 1 }\n\
                   func bar() -> lang.i64 { foo() }\n";
        let f = c.set_source("/tmp/rename_edit.ks", src.into());
        c.build(f);

        let foo = find_decl(c.world(), f, "foo");
        let target = Target::Entity(foo);
        let sources = sources_of("/tmp/rename_edit.ks", src);
        let mut sites = collect_sites(c.world(), c.root(), &target);
        push_decl_site(c.world(), c.root(), &target, &sources, &mut sites);

        let edit = build_workspace_edit(c.world(), &sources, &sites, "renamed");
        let changes = edit.changes.expect("changes present");
        assert_eq!(changes.len(), 1, "edits in one file");
        let (_, edits) = changes.iter().next().unwrap();
        // Expect: declaration site + call site = at least 2 distinct edits.
        assert!(edits.len() >= 2, "expected ≥2 edits, got {}", edits.len());
        for e in edits {
            assert_eq!(e.new_text, "renamed");
        }
    }
}
