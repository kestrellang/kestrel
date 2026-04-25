//! `textDocument/prepareRename` and `textDocument/rename`.
//!
//! Both methods share the same dispatch as `references.rs`: locate the
//! entity (or local) at the cursor. `prepareRename` returns the identifier
//! range + current text as the popup placeholder. `rename` validates the new
//! name with the lexer, runs `references_to` for use sites, performs a
//! per-site collision check via `ResolveName`, and bundles the resulting
//! `TextEdit`s into a `WorkspaceEdit`.

use std::collections::HashMap;

use kestrel_ast_builder::{CstNode, DeclSpan, FileId, FilePath, Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_lexer::{lex, Token};
use kestrel_name_res::{NameResolution, ResolveName};
use kestrel_span::Span;
use kestrel_syntax_tree::utils::get_name_span;
use kestrel_type_infer::InferBody;
use tower_lsp::jsonrpc::{Error as RpcError, ErrorCode};
use tower_lsp::lsp_types::{
    PrepareRenameResponse, RenameParams, TextDocumentPositionParams, TextEdit, Url, WorkspaceEdit,
};

use crate::position::LineIndex;
use crate::references::{self, RefKind, ReferenceSite};
use crate::semantic;
use crate::server::{path_to_url, rebuild_compiler, url_to_path, SharedState};

// ===== prepareRename =====

pub async fn prepare(
    state: SharedState,
    params: TextDocumentPositionParams,
) -> Result<Option<PrepareRenameResponse>, RpcError> {
    let uri = params.text_document.uri;
    let pos = params.position;
    let path = url_to_path(&uri);

    let (sources, line_index) = {
        let s = state.lock().await;
        let Some(li) = s.docs.get(&uri).map(|d| d.line_index.clone()) else {
            return Ok(None);
        };
        (s.sources.clone(), li)
    };
    let offset = line_index.position_to_offset(pos);

    let result = tokio::task::spawn_blocking(move || -> Option<PrepareRenameResponse> {
        let (compiler, _) = rebuild_compiler(&sources);
        let file_entity = semantic::file_entity_for_path(&compiler, &path)?;
        let world = compiler.world();
        let root = compiler.root();

        let target = target_at(world, file_entity, offset, root)?;
        let (placeholder, span) = identifier_for_target(world, root, &target)?;
        let li = LineIndex::new(sources.get(&path)?.clone());
        let range = li.range_for(span.start, span.end);

        Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder })
    })
    .await
    .ok()
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

    let (sources, line_index) = {
        let s = state.lock().await;
        let Some(li) = s.docs.get(&uri).map(|d| d.line_index.clone()) else {
            return Ok(None);
        };
        (s.sources.clone(), li)
    };
    let offset = line_index.position_to_offset(pos);

    let outcome = tokio::task::spawn_blocking(
        move || -> Result<Option<WorkspaceEdit>, RpcError> {
            let (compiler, _) = rebuild_compiler(&sources);
            let file_entity = match semantic::file_entity_for_path(&compiler, &path) {
                Some(f) => f,
                None => return Ok(None),
            };
            let world = compiler.world();
            let root = compiler.root();

            let target = match target_at(world, file_entity, offset, root) {
                Some(t) => t,
                None => return Ok(None),
            };

            // Stdlib / overload-set guard. identifier_for_target returns None
            // for both, so prepareRename already filters most of these — but
            // a client can call rename without prepareRename, so re-check.
            if identifier_for_target(world, root, &target).is_none() {
                return Err(RpcError {
                    code: ErrorCode::InvalidRequest,
                    message: "this symbol cannot be renamed".into(),
                    data: None,
                });
            }

            let mut sites = collect_sites(world, root, &target);

            // Add the declaration site itself so its text changes too.
            push_decl_site(world, root, &target, &mut sites);

            check_collisions(world, root, &target, &new_name, &sites)?;

            let edit = build_workspace_edit(world, &sources, &sites, &new_name);
            Ok(Some(edit))
        },
    )
    .await
    .map_err(|_| RpcError::internal_error())??;

    Ok(outcome)
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
        }) {
            if let Some(expr_id) = semantic::hir_expr_at(&hir, offset) {
                if let Some(t) = resolve_expr(&hir, body_entity, expr_id, &ctx, root) {
                    return Some(t);
                }
            }
        }
    }
    let decl = semantic::enclosing_decl_at(world, file_entity, offset)?;
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

/// Get the identifier text + span we'd rename for a target. Returns `None`
/// for targets we refuse to rename: stdlib entities (no source span we can
/// edit) and overload sets (would need to fix every overload).
fn identifier_for_target(world: &World, root: Entity, target: &Target) -> Option<(String, Span)> {
    match target {
        Target::Entity(e) => {
            // Reject stdlib entities — they have no FilePath ancestor.
            entity_file(world, *e)?;
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
            Some((name, span))
        },
        Target::Local { body, id } => {
            let ctx = world.query_context();
            let hir = ctx.query(LowerBody {
                entity: *body,
                root,
            })?;
            let local = &hir.locals[*id];
            Some((local.name.clone(), local.span.clone()))
        },
    }
}

fn collect_sites(world: &World, root: Entity, target: &Target) -> Vec<ReferenceSite> {
    match target {
        Target::Entity(e) => references::references_to(world, root, *e),
        Target::Local { body, id } => references::local_references(world, *body, root, *id),
    }
}

fn push_decl_site(world: &World, root: Entity, target: &Target, sites: &mut Vec<ReferenceSite>) {
    if let Some((_, span)) = identifier_for_target(world, root, target) {
        let file = match target {
            Target::Entity(e) => entity_file(world, *e),
            Target::Local { body, .. } => entity_file(world, *body),
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

fn entity_file(world: &World, entity: Entity) -> Option<Entity> {
    if let Some(fid) = world.get::<FileId>(entity) {
        return Some(fid.0);
    }
    let mut cur = world.parent_of(entity);
    while let Some(e) = cur {
        if let Some(fid) = world.get::<FileId>(e) {
            return Some(fid.0);
        }
        cur = world.parent_of(e);
    }
    None
}

// ===== Validation =====

fn validate_identifier(s: &str) -> Result<(), &'static str> {
    if s.is_empty() {
        return Err("rename target cannot be empty");
    }
    let mut tokens = lex(s, 0).filter(|t| match t {
        Ok(spanned) => !matches!(
            spanned.value,
            Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
        ),
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
            !entities.iter().any(|&e| e == target)
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

fn clip_to_identifier(source: &str, span: &Span, kind: RefKind) -> Span {
    if matches!(kind, RefKind::Direct) {
        return span.clone();
    }
    let text = match source.get(span.start..span.end) {
        Some(t) => t,
        None => return span.clone(),
    };
    let trailing_start_in_text = text
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident_char(*c))
        .last()
        .map(|(i, _)| i);
    match trailing_start_in_text {
        Some(start) => Span::new(span.file_id, span.start + start..span.end),
        None => span.clone(),
    }
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn find_decl(world: &World, file: Entity, name: &str) -> Entity {
        for (e, n) in world.iter_component::<Name>() {
            if n.0 != name {
                continue;
            }
            if let Some(fid) = world.get::<FileId>(e) {
                if fid.0 == file {
                    return e;
                }
            }
        }
        panic!("no decl `{name}` in file");
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
        let mut sites = collect_sites(c.world(), c.root(), &target);
        push_decl_site(c.world(), c.root(), &target, &mut sites);

        let mut sources = HashMap::new();
        sources.insert("/tmp/rename_edit.ks".to_string(), src.to_string());

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
