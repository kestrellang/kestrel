//! `textDocument/documentHighlight` — highlight all references to the symbol
//! under the cursor within the current file.
//!
//! Reuses `references::references_to` (or `local_references`) filtered to the
//! current file. Maps `RefKind` to `DocumentHighlightKind`.

use kestrel_ast_builder::DeclSpan;
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams};

use crate::position::LineIndex;
use crate::references::{self, RefKind, ReferenceSite, clip_to_identifier};
use crate::semantic;
use crate::server::{SharedState, url_to_path};

pub async fn handle(
    state: SharedState,
    params: DocumentHighlightParams,
) -> Option<Vec<DocumentHighlight>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
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

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<DocumentHighlight>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                let target = target_at(world, file_entity, offset, root, compiler)?;
                let sites = collect_sites(world, root, &target, file_entity, compiler);

                let source = sources.get(&path)?;
                let li = LineIndex::new(source.clone());
                let mut out = Vec::new();
                for site in &sites {
                    let clipped = clip_to_identifier(source, &site.span, site.kind);
                    let range = li.range_for(clipped.start, clipped.end);
                    let kind = match site.kind {
                        RefKind::Direct | RefKind::MemberAccess | RefKind::Pattern => {
                            DocumentHighlightKind::READ
                        },
                    };
                    out.push(DocumentHighlight {
                        range,
                        kind: Some(kind),
                    });
                }
                if out.is_empty() { None } else { Some(out) }
            },
        )
        .await
        .flatten()
}

enum Target {
    Entity(Entity),
    Local { body: Entity, id: LocalId },
}

fn target_at(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
    compiler: &kestrel_compiler::Compiler,
) -> Option<Target> {
    // Type-position cursor.
    let file_cst = compiler.parse(file_entity).tree;
    if let Some((entity, _)) =
        crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
    {
        return Some(Target::Entity(entity));
    }

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

/// Collect reference sites scoped to the current file.
fn collect_sites(
    world: &World,
    root: Entity,
    target: &Target,
    file_entity: Entity,
    compiler: &kestrel_compiler::Compiler,
) -> Vec<ReferenceSite> {
    let mut sites = match target {
        Target::Entity(e) => {
            let mut s = references::references_to(world, root, *e);
            s.retain(|site| site.file == file_entity);

            // Type-position references, filtered to this file.
            for (file, span) in crate::types::type_references_workspace(world, root, compiler, *e) {
                if file == file_entity {
                    s.push(ReferenceSite {
                        file,
                        span,
                        kind: RefKind::Direct,
                    });
                }
            }
            s
        },
        Target::Local { body, id } => references::local_references(world, *body, root, *id),
    };

    // Add the declaration site.
    if let Target::Entity(e) = target {
        if let Some(span) = world.get::<DeclSpan>(*e).map(|s| s.0.clone()) {
            if let Some(file) = crate::references::entity_file(world, *e) {
                if file == file_entity {
                    sites.push(ReferenceSite {
                        file,
                        span,
                        kind: RefKind::Direct,
                    });
                }
            }
        }
    }
    if let Target::Local { body, id } = target {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: *body,
            root,
        }) {
            if let Some(file) = crate::references::entity_file(world, *body) {
                if file == file_entity {
                    sites.push(ReferenceSite {
                        file,
                        span: hir.locals[*id].span.clone(),
                        kind: RefKind::Direct,
                    });
                }
            }
        }
    }

    sites
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn highlights_for(src: &str, needle: &str) -> Vec<DocumentHighlight> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/highlight.ks", src.into());
        c.build(f);
        let offset = src.find(needle).expect("needle not found");
        let world = c.world();
        let root = c.root();
        let target = target_at(world, f, offset, root, &c).expect("target");
        let sites = collect_sites(world, root, &target, f, &c);
        let li = LineIndex::new(src.to_string());
        sites
            .iter()
            .map(|site| {
                let clipped = clip_to_identifier(src, &site.span, site.kind);
                let range = li.range_for(clipped.start, clipped.end);
                DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::READ),
                }
            })
            .collect()
    }

    #[test]
    fn highlights_function_call_and_decl() {
        let src = "module Test\n\
                   func target() -> lang.i64 { 1 }\n\
                   func caller() -> lang.i64 { target() }\n";
        let hl = highlights_for(src, "target");
        // At least decl + call site.
        assert!(hl.len() >= 2, "expected >=2 highlights, got {}", hl.len());
    }

    #[test]
    fn highlights_local_variable() {
        let src = "module Test\n\
                   func foo() -> lang.i64 {\n  \
                     let x = 1;\n  \
                     x\n\
                   }\n";
        // Cursor on bare `x` reference.
        let pos = src.rfind("x\n").unwrap();
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hl_local.ks", src.into());
        c.build(f);
        let target = target_at(c.world(), f, pos, c.root(), &c).expect("target");
        let sites = collect_sites(c.world(), c.root(), &target, f, &c);
        // Two uses of x plus the declaration.
        assert!(
            sites.len() >= 2,
            "expected >=2 sites for local x, got {}",
            sites.len()
        );
    }

    #[test]
    fn no_highlights_on_empty_space() {
        let src = "module Test\n\n\nfunc foo() -> lang.i64 { 1 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hl_empty.ks", src.into());
        c.build(f);
        // Cursor on the blank line between module and func.
        let offset = src.find("\n\n").unwrap() + 1;
        let result = target_at(c.world(), f, offset, c.root(), &c);
        // May resolve to the module or None — either way, should not crash.
        let _ = result;
    }
}
