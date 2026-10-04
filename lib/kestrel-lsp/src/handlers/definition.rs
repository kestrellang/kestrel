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
use kestrel_hir::res::LocalId;
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
                let file_cst = compiler.parse(file_entity).tree();
                if let Some((entity, _span)) =
                    crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
                    && let Some(loc) = entity_location(world, &sources, entity)
                {
                    return Some(loc);
                }

                // A local's own declaring identifier is its definition.
                if let Some((body, local)) =
                    semantic::local_declared_at(world, root, file_entity, offset)
                {
                    return local_location(world, &sources, root, body, local);
                }

                let body_entity = semantic::body_entity_at(world, file_entity, offset)?;
                let ctx = world.query_context();
                let lowered = semantic::lowered_body(world, root, body_entity)?;
                let typed = ctx.query(InferBody {
                    entity: body_entity,
                    root,
                })?;

                let expr_id = semantic::expr_at(world, body_entity, &lowered, offset)?;
                match resolve_target(&lowered.body, &typed, expr_id, offset)? {
                    Target::Entity(entity) => entity_location(world, &sources, entity),
                    Target::Local(local) => {
                        local_location(world, &sources, root, body_entity, local)
                    },
                }
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
    /// A local of the cursor's body.
    Local(LocalId),
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
        HirExpr::Local(local_id, _) => Some(Target::Local(*local_id)),
        HirExpr::MethodCall { .. }
        | HirExpr::Field { .. }
        | HirExpr::Call { .. }
        | HirExpr::ProtocolCall { .. } => {
            typed.resolutions.get(&expr_id).copied().map(Target::Entity)
        },
        _ => None,
    }
}

/// The location of `entity`'s declaration.
fn entity_location(
    world: &kestrel_hecs::World,
    sources: &HashMap<String, String>,
    entity: Entity,
) -> Option<(Url, Range)> {
    let span = world.get::<DeclSpan>(entity)?.0.clone();
    let file_entity = crate::references::entity_file(world, entity)?;
    span_location(world, sources, file_entity, &span)
}

/// The location of the identifier that declares `local` (of `body`), from
/// the body's source map. `None` for a local the source does not spell
/// (`self`, an implicit `it`, desugaring temporaries).
fn local_location(
    world: &kestrel_hecs::World,
    sources: &HashMap<String, String>,
    root: Entity,
    body: Entity,
    local: LocalId,
) -> Option<(Url, Range)> {
    let (file, span) = semantic::local_name_site(world, root, body, local)?;
    span_location(world, sources, file, &span)
}

fn span_location(
    world: &kestrel_hecs::World,
    sources: &HashMap<String, String>,
    file: Entity,
    span: &kestrel_span::Span,
) -> Option<(Url, Range)> {
    let file_path = world.get::<FilePath>(file).map(|p| p.0.clone())?;
    let url = path_to_url(&file_path)?;
    let source = sources.get(&file_path)?;
    let li = LineIndex::new(source.clone());
    Some((url, li.range_for(span.start, span.end)))
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
        let lowered = semantic::lowered_body(world, c.root(), body).expect("hir");
        let typed = ctx
            .query(InferBody {
                entity: body,
                root: c.root(),
            })
            .expect("typed");
        let target_name = |offset: usize| {
            let expr = semantic::expr_at(world, body, &lowered, offset).expect("expr");
            match resolve_target(&lowered.body, &typed, expr, offset) {
                Some(Target::Entity(e)) => world.get::<Name>(e).map(|n| n.0.clone()),
                _ => None,
            }
        };
        assert_eq!(target_name(at + "B.".len()), Some("Item".into()));
        assert_eq!(target_name(at), Some("B".into()));
    }

    /// Go-to-definition on a local lands on its name: the `let` binding's
    /// identifier (not the statement), a parameter's name in the signature
    /// (not offset 0 of some file), a closure parameter's name.
    #[test]
    fn definition_on_local_lands_on_its_name() {
        let src = "module T\n\
                   func apply(g: (lang.i64) -> lang.i64) -> lang.i64 { g(1) }\n\
                   func f(n: lang.i64) -> lang.i64 {\n\
                   \x20   let x = n;\n\
                   \x20   apply({ (k) in lang.i64_add(k, x) })\n\
                   }\n";
        let mut c = Compiler::new();
        let path = "/tmp/def_local.ks";
        let f = c.set_source(path, src.into());
        c.build(f);
        let world = c.world();
        let root = c.root();
        let mut sources = HashMap::new();
        sources.insert(path.to_string(), src.to_string());
        let li = LineIndex::new(src.to_string());

        let definition_of = |use_at: usize| -> &str {
            let body = semantic::body_entity_at(world, f, use_at).expect("body");
            let lowered = semantic::lowered_body(world, root, body).expect("hir");
            let typed = world
                .query_context()
                .query(InferBody { entity: body, root })
                .expect("typed");
            let expr = semantic::expr_at(world, body, &lowered, use_at).expect("expr");
            let Some(Target::Local(local)) = resolve_target(&lowered.body, &typed, expr, use_at)
            else {
                panic!("not a local at {use_at}");
            };
            let (_, range) = local_location(world, &sources, root, body, local).expect("location");
            let (start, end) = (
                li.position_to_offset(range.start),
                li.position_to_offset(range.end),
            );
            assert!(start > 0, "a definition at the top of the file");
            &src[start..end]
        };
        let x_use = src.rfind("x)").unwrap();
        assert_eq!(definition_of(x_use), "x");
        let x_decl = src.find("let x").unwrap() + "let ".len();
        assert_eq!(&src[x_decl..x_decl + 1], "x");

        let n_use = src.find("= n;").unwrap() + "= ".len();
        assert_eq!(definition_of(n_use), "n");
        let k_use = src.find("(k, x)").unwrap() + 1;
        assert_eq!(definition_of(k_use), "k");

        // A local's own name is its definition.
        let (body, local) =
            semantic::local_declared_at(world, root, f, x_decl).expect("declared here");
        let (_, range) = local_location(world, &sources, root, body, local).unwrap();
        assert_eq!(li.position_to_offset(range.start), x_decl);
    }
}
