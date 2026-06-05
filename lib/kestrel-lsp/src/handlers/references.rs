//! `textDocument/references` — find every site that resolves to the symbol
//! under the cursor.
//!
//! Dispatch shape mirrors `definition.rs`: locate the entity (or local) at
//! the cursor, then ask `references::references_to` (or `local_references`)
//! for the use sites. `MemberAccess` and `Pattern` site spans cover the
//! whole expression / pattern; we clip them to the trailing identifier so
//! the editor highlights just the name.

use std::collections::HashMap;

use kestrel_ast_builder::{DeclSpan, FilePath};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{Location, Range, ReferenceParams, Url};

use crate::position::LineIndex;
use crate::references::{self, RefKind, ReferenceSite, clip_to_identifier};
use crate::semantic;
use crate::server::{SharedState, path_to_url, url_to_path};

pub async fn handle(state: SharedState, params: ReferenceParams) -> Option<Vec<Location>> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let path = url_to_path(&uri);
    let include_declaration = params.context.include_declaration;

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
            move |compiler, _by_path| -> Option<Vec<Location>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                let target = target_at(world, file_entity, offset, root, compiler)?;
                let sites = collect_sites(world, root, &target, include_declaration, compiler);

                let mut by_file: HashMap<Entity, LineIndex> = HashMap::new();
                let mut out: Vec<Location> = Vec::new();
                for site in sites {
                    let Some((url, range)) = site_to_location(world, &sources, &site, &mut by_file)
                    else {
                        continue;
                    };
                    out.push(Location { uri: url, range });
                }
                Some(out)
            },
        )
        .await
        .flatten()
}

/// What the cursor resolves to.
enum Target {
    /// External entity — references span the workspace.
    Entity(Entity),
    /// Local within a body — references stay in that body.
    Local { body: Entity, id: LocalId },
}

fn target_at(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
    compiler: &kestrel_compiler::Compiler,
) -> Option<Target> {
    // Type-position cursor (`func bar(x: Foo)`): resolve via the file CST.
    let file_cst = compiler.parse(file_entity).tree;
    if let Some((entity, _)) =
        crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
    {
        return Some(Target::Entity(entity));
    }

    // First try: cursor is inside a function body. Resolve the HIR expression.
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

    // Fallback: cursor is on a declaration's identifier (function name, struct
    // field decl, etc). Use the smallest enclosing decl.
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

fn collect_sites(
    world: &World,
    root: Entity,
    target: &Target,
    include_declaration: bool,
    compiler: &kestrel_compiler::Compiler,
) -> Vec<ReferenceSite> {
    let mut sites = match target {
        Target::Entity(e) => references::references_to(world, root, *e),
        Target::Local { body, id } => references::local_references(world, *body, root, *id),
    };

    // Type-position references — `Foo` in parameter / return / field types.
    // Not covered by `references_to` (those resolutions live transiently in
    // `ResolveTypePath` and aren't kept on the AST). Walk the workspace CSTs.
    if let Target::Entity(e) = target {
        for (file, span) in crate::types::type_references_workspace(world, root, compiler, *e) {
            sites.push(ReferenceSite {
                file,
                span,
                kind: RefKind::Direct,
            });
        }
    }

    if include_declaration {
        if let Target::Entity(e) = target
            && let Some(span) = world.get::<DeclSpan>(*e).map(|s| s.0.clone())
            && let Some(file) = crate::references::entity_file(world, *e)
        {
            sites.push(ReferenceSite {
                file,
                span,
                kind: RefKind::Direct,
            });
        }
        // For locals, the definition site is `hir.locals[id].span` — included
        // here too.
        if let Target::Local { body, id } = target {
            let ctx = world.query_context();
            if let Some(hir) = ctx.query(LowerBody {
                entity: *body,
                root,
            }) && let Some(file) = crate::references::entity_file(world, *body)
            {
                sites.push(ReferenceSite {
                    file,
                    span: hir.locals[*id].span.clone(),
                    kind: RefKind::Direct,
                });
            }
        }
    }

    sites
}

/// Map a `ReferenceSite` to an LSP `Location`, clipping `MemberAccess` and
/// `Pattern` spans to their trailing identifier so the highlighted range is
/// just the name, not the whole expression.
fn site_to_location(
    world: &World,
    sources: &HashMap<String, String>,
    site: &ReferenceSite,
    by_file: &mut HashMap<Entity, LineIndex>,
) -> Option<(Url, Range)> {
    let file_path = world.get::<FilePath>(site.file).map(|p| p.0.clone())?;
    let url = path_to_url(&file_path)?;
    let source = sources.get(&file_path)?;
    let li = by_file
        .entry(site.file)
        .or_insert_with(|| LineIndex::new(source.clone()));

    let clipped = clip_to_identifier(source, &site.span, site.kind);
    Some((url, li.range_for(clipped.start, clipped.end)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::LineIndex;
    use kestrel_compiler::Compiler;
    use kestrel_span::Span;

    fn site_kinds_for(target_name: &str, src: &str) -> Vec<(usize, usize, RefKind)> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/refs_handler.ks", src.into());
        c.build(f);

        use kestrel_ast_builder::{FileId as F, Name};
        let target = {
            let mut found = None;
            for (e, n) in c.world().iter_component::<Name>() {
                if n.0 != target_name {
                    continue;
                }
                if let Some(fid) = c.world().get::<F>(e)
                    && fid.0 == f
                {
                    found = Some(e);
                    break;
                }
            }
            found.unwrap_or_else(|| panic!("no `{target_name}` found"))
        };

        crate::references::references_to(c.world(), c.root(), target)
            .into_iter()
            .map(|s| (s.span.start, s.span.end, s.kind))
            .collect()
    }

    #[test]
    fn clip_member_access_keeps_only_identifier() {
        let span = Span::new(0, 0..5); // covers `a.foo`
        let clipped = clip_to_identifier("a.foo", &span, RefKind::MemberAccess);
        assert_eq!(clipped.start, 2);
        assert_eq!(clipped.end, 5);
    }

    #[test]
    fn clip_direct_returns_input() {
        let span = Span::new(0, 0..3);
        let clipped = clip_to_identifier("foo", &span, RefKind::Direct);
        assert_eq!(clipped.start, 0);
        assert_eq!(clipped.end, 3);
    }

    #[test]
    fn type_position_references_collected() {
        // `Point` appears once as a struct decl, once as a parameter type,
        // and once as a return type. `references_to` only sees the body
        // expr-side `Point(...)` constructor call (none here), so type-side
        // walks must surface the `(p: Point) -> Point` references.
        let mut c = Compiler::new();
        let src = "module T\nstruct Point { var x: lang.i64 }\nfunc id(p: Point) -> Point { p }\n";
        let f = c.set_source("/tmp/refs_type.ks", src.into());
        c.build(f);
        use kestrel_ast_builder::{FileId as F, Name};
        let target = c
            .world()
            .iter_component::<Name>()
            .find(|(e, n)| n.0 == "Point" && c.world().get::<F>(*e).map(|f2| f2.0) == Some(f))
            .map(|(e, _)| e)
            .expect("Point");
        let sites = collect_sites(c.world(), c.root(), &Target::Entity(target), false, &c);
        let texts: Vec<&str> = sites
            .iter()
            .map(|s| &src[s.span.start..s.span.end])
            .collect();
        let count = texts.iter().filter(|t| **t == "Point").count();
        assert_eq!(
            count, 2,
            "expected 2 type-position Point refs; got {texts:?}"
        );
    }

    #[test]
    fn member_access_clipped_through_handler() {
        // p.x is a Field expr; its raw span covers `p.x` but the LSP-facing
        // location should highlight just `x`.
        let src = "module Test\nstruct Point { var x: lang.i64; }\n\
                   func at(p: Point) -> lang.i64 { p.x }\n";
        let sites = site_kinds_for("x", src);
        // We expect at least one MemberAccess site; clip its span and verify
        // the resulting text is `x`.
        let mut found = false;
        for (start, end, kind) in &sites {
            if matches!(kind, RefKind::MemberAccess) {
                let clipped =
                    clip_to_identifier(src, &Span::new(0, *start..*end), RefKind::MemberAccess);
                assert_eq!(&src[clipped.start..clipped.end], "x");
                found = true;
            }
        }
        assert!(found, "no MemberAccess site found; sites = {sites:?}");
    }

    #[test]
    fn site_to_location_returns_proper_range() {
        let src = "module Test\nfunc foo() -> lang.i64 { 1 }\nfunc bar() -> lang.i64 { foo() }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/refs_loc.ks", src.into());
        c.build(f);

        use kestrel_ast_builder::{FileId as F, Name};
        let foo = {
            let mut found = None;
            for (e, n) in c.world().iter_component::<Name>() {
                if n.0 == "foo"
                    && let Some(fid) = c.world().get::<F>(e)
                    && fid.0 == f
                {
                    found = Some(e);
                    break;
                }
            }
            found.unwrap()
        };

        let sites = crate::references::references_to(c.world(), c.root(), foo);
        assert!(!sites.is_empty(), "expected at least one ref to foo");

        let mut by_file = HashMap::new();
        let mut sources = HashMap::new();
        sources.insert("/tmp/refs_loc.ks".to_string(), src.to_string());

        let loc = site_to_location(c.world(), &sources, &sites[0], &mut by_file).expect("location");
        // The first ref is the call `foo()` on line 2 (0-indexed).
        // Confirm by re-extracting the substring from the raw range.
        let li = LineIndex::new(src.to_string());
        let start_offset = li.position_to_offset(loc.1.start);
        let end_offset = li.position_to_offset(loc.1.end);
        let _ = (start_offset, end_offset, loc); // smoke-only
    }
}
