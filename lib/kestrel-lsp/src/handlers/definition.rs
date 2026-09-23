//! `textDocument/definition` — jump from a name to its declaration.
//!
//! Three sources of resolution, in order:
//!
//! 1. **HIR `Def`/`Local`/method-call resolutions** — when the cursor lands on
//!    an expression inside a body, the inferred body already knows what each
//!    name refers to (`HirExpr::Def(entity, ...)`, `HirExpr::Local(local)`,
//!    `TypedBody::resolutions[expr_id]` for `MethodCall` / `Field` / call).
//! 2. **`ResolveName` / `ResolveValuePath`** — for identifiers in declaration
//!    positions where there's no body context (return types, field types).
//!    Not implemented in this first cut.
//! 3. **None** — if we can't find a target, return no locations.

use std::collections::HashMap;

use kestrel_ast_builder::{DeclSpan, FilePath};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir::res::Local;
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Url};

use crate::position::LineIndex;
use crate::semantic;
use crate::server::{SharedState, path_to_url, url_to_path};

pub async fn handle(
    state: SharedState,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            line_index,
        )
    };
    let offset = line_index.position_to_offset(pos);

    let result = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<(Url, Range)> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                // Type-position cursor (`func bar(x: Foo)`): resolve via CST before
                // falling into the body-based path. The body lookup wouldn't find
                // anything for type positions because they don't appear in HIR exprs.
                let file_cst = compiler.parse(file_entity).tree;
                if let Some((entity, _span)) =
                    crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
                    && let Some(loc) = target_to_location(world, &sources, Target::Entity(entity))
                {
                    return Some(loc);
                }

                let body_entity = semantic::body_entity_at(world, file_entity, offset)?;
                let ctx = world.query_context();
                let hir = ctx.query(LowerBody {
                    entity: body_entity,
                    root,
                })?;
                let typed = ctx.query(InferBody {
                    entity: body_entity,
                    root,
                })?;

                let expr_id = semantic::hir_expr_at(&hir, offset)?;
                let target = resolve_target(&hir, &typed, expr_id, offset)?;

                target_to_location(world, &sources, target)
            },
        )
        .await??;

    let (uri, range) = result;
    Some(GotoDefinitionResponse::Scalar(Location { uri, range }))
}

use tower_lsp::lsp_types::Range;

/// What the cursor's expression points at.
enum Target {
    /// External entity declaration — use its `DeclSpan` + `FileId`.
    Entity(Entity),
    /// Local definition site within the same body.
    Local { span: kestrel_span::Span },
}

fn resolve_target(
    hir: &HirBody,
    typed: &kestrel_type_infer::result::TypedBody,
    expr_id: HirExprId,
    offset: usize,
) -> Option<Target> {
    match &hir.exprs[expr_id] {
        HirExpr::Def(entity, _, _) => Some(Target::Entity(*entity)),
        // A type in expression position: the segment under the cursor.
        HirExpr::TypeRef { ty, .. } => {
            semantic::hir_ty_entity_at(ty, offset).map(|(e, _)| Target::Entity(e))
        },
        HirExpr::Local(local_id, _) => {
            let local: &Local = &hir.locals[*local_id];
            Some(Target::Local {
                span: local.span.clone(),
            })
        },
        HirExpr::MethodCall { .. }
        | HirExpr::Field { .. }
        | HirExpr::Call { .. }
        | HirExpr::ProtocolCall { .. } => {
            typed.resolutions.get(&expr_id).copied().map(Target::Entity)
        },
        _ => None,
    }
}

fn target_to_location(
    world: &kestrel_hecs::World,
    sources: &HashMap<String, String>,
    target: Target,
) -> Option<(Url, Range)> {
    match target {
        Target::Entity(entity) => {
            let span = world.get::<DeclSpan>(entity)?.0.clone();
            let file_entity = crate::references::entity_file(world, entity)?;
            let file_path = world.get::<FilePath>(file_entity).map(|p| p.0.clone())?;
            let url = path_to_url(&file_path)?;
            let source = sources.get(&file_path)?;
            let li = LineIndex::new(source.clone());
            Some((url, li.range_for(span.start, span.end)))
        },
        Target::Local { span } => {
            // Walk world to find the entity whose index matches span.file_id.
            for (e, fp) in world.iter_component::<FilePath>() {
                if e.index() == span.file_id {
                    let url = path_to_url(&fp.0)?;
                    let source = sources.get(&fp.0)?;
                    let li = LineIndex::new(source.clone());
                    return Some((url, li.range_for(span.start, span.end)));
                }
            }
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_ast_builder::Name;
    use kestrel_compiler::Compiler;

    /// G26: go-to-definition on `Item` in `B.Item.zero()` lands on the
    /// associated type, and on `B` lands on the type parameter — the
    /// receiver is a `TypeRef`, which carries both.
    #[test]
    fn definition_on_projection_receiver_segments() {
        let mut c = Compiler::new();
        let src = "module T\n\
                   protocol Zero { static func zero() -> Self }\n\
                   protocol Producer { type Item; func produce() -> Item }\n\
                   func make[B](b: B) -> B.Item where B: Producer, B.Item: Zero { B.Item.zero() }\n";
        let f = c.set_source("/tmp/def_typeref.ks", src.into());
        c.build(f);
        let at = src.find("B.Item.zero").unwrap();
        let world = c.world();
        let body = semantic::body_entity_at(world, f, at).expect("body");
        let ctx = world.query_context();
        let hir = ctx
            .query(LowerBody {
                entity: body,
                root: c.root(),
            })
            .expect("hir");
        let typed = ctx
            .query(InferBody {
                entity: body,
                root: c.root(),
            })
            .expect("typed");
        let target_name = |offset: usize| {
            let expr = semantic::hir_expr_at(&hir, offset).expect("expr");
            match resolve_target(&hir, &typed, expr, offset) {
                Some(Target::Entity(e)) => world.get::<Name>(e).map(|n| n.0.clone()),
                _ => None,
            }
        };
        assert_eq!(target_name(at + "B.".len()), Some("Item".into()));
        assert_eq!(target_name(at), Some("B".into()));
    }
}
