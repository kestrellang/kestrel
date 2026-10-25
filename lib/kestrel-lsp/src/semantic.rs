//! Bridge between cursor positions and compiler entities / HIR nodes.
//!
//! Most of M2's "what's at the cursor?" logic funnels through these helpers
//! so handlers stay narrow and the lookup rules stay in one place.
//!
//! Positions are never matched against spans here. A body's
//! [`BodySourceMap`](kestrel_hir_lower::BodySourceMap) (from
//! `LowerBodyWithSourceMap`) answers "which HIR id is at this offset" and
//! "where is this local declared"; a declaration's `CstNode` pointer answers
//! "which declaration is this node". Both are produced by the passes that
//! built the ids, so the answers are exact.

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_ast_builder::{CstNode, FileId, FilePath, NodeKind, Valued};
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId, HirPat, HirPatId};
use kestrel_hir::res::LocalId;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerBodyWithSourceMap, LoweredBody};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxNode, SyntaxNodePtr};
use kestrel_type_infer::InferBody;
use rowan::TextSize;

/// Look up the file entity for a compiler-key path.
pub fn file_entity_for_path(compiler: &kestrel_compiler::Compiler, path: &str) -> Option<Entity> {
    compiler.files().get(path).copied()
}

/// Find the on-disk path of the file that contains `entity`.
pub fn entity_file_path(world: &World, entity: Entity) -> Option<String> {
    if let Some(p) = world.get::<FilePath>(entity) {
        return Some(p.0.clone());
    }
    let fid = world.get::<FileId>(entity)?;
    world.get::<FilePath>(fid.0).map(|p| p.0.clone())
}

/// Smallest entity with a `Valued` body whose CST range contains `offset` and
/// whose `FileId` matches `file_entity`. This is the entity we feed into
/// `LowerBody` / `InferBody`.
pub fn body_entity_at(world: &World, file_entity: Entity, offset: usize) -> Option<Entity> {
    let pos = TextSize::from(offset as u32);
    let mut best: Option<(Entity, u32)> = None;
    for (entity, valued) in world.iter_component::<Valued>() {
        let Some(fid) = world.get::<FileId>(entity) else {
            continue;
        };
        if fid.0 != file_entity {
            continue;
        }
        let range = valued.0.text_range();
        if range.start() <= pos && pos <= range.end() {
            let len: u32 = range.len().into();
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((entity, len));
            }
        }
    }
    best.map(|(e, _)| e)
}

/// Get the byte span of an HIR expression. HirExpr doesn't carry a `span()`
/// method, so we destructure each variant. Keep this in sync with the enum.
pub fn hir_expr_span(expr: &HirExpr) -> Span {
    match expr {
        HirExpr::Literal { span, .. } => span.clone(),
        HirExpr::Tuple { span, .. } => span.clone(),
        HirExpr::Array { span, .. } => span.clone(),
        HirExpr::Dict { span, .. } => span.clone(),
        HirExpr::Closure { span, .. } => span.clone(),
        HirExpr::Local(_, span) => span.clone(),
        HirExpr::Def(_, _, span) => span.clone(),
        HirExpr::TypeRef { span, .. } => span.clone(),
        HirExpr::OverloadSet { span, .. } => span.clone(),
        HirExpr::Borrow { span, .. } => span.clone(),
        HirExpr::Field { span, .. } => span.clone(),
        HirExpr::TupleIndex { span, .. } => span.clone(),
        HirExpr::ImplicitMember { span, .. } => span.clone(),
        HirExpr::Call { span, .. } => span.clone(),
        HirExpr::MethodCall { span, .. } => span.clone(),
        HirExpr::ProtocolCall { span, .. } => span.clone(),
        HirExpr::If { span, .. } => span.clone(),
        HirExpr::Loop { span, .. } => span.clone(),
        HirExpr::Match { span, .. } => span.clone(),
        HirExpr::Break { span, .. } => span.clone(),
        HirExpr::Continue { span, .. } => span.clone(),
        HirExpr::Return { span, .. } => span.clone(),
        HirExpr::Assign { span, .. } => span.clone(),
        HirExpr::Block { span, .. } => span.clone(),
        HirExpr::Error { span } => span.clone(),
        HirExpr::Sugar { span, .. } => span.clone(),
    }
}

/// Every entity a `HirTy` names, each paired with the span of the type node
/// that names it. `B.Item` gives `(B, "B")` and `(Producer.Item, "B.Item")`:
/// a projection's own span covers its base, so callers that want only the
/// trailing name clip with `references::clip_to_identifier`.
///
/// The one walk for a type written in expression position
/// (`HirExpr::TypeRef`, G26): hover, go-to-definition and references all
/// read it from here.
pub fn hir_ty_named_entities(ty: &HirTy) -> Vec<(Entity, Span)> {
    let mut out = Vec::new();
    collect_hir_ty_entities(ty, &mut out);
    out
}

fn collect_hir_ty_entities(ty: &HirTy, out: &mut Vec<(Entity, Span)>) {
    match ty {
        HirTy::Struct { entity, args, span }
        | HirTy::Enum { entity, args, span }
        | HirTy::Protocol { entity, args, span }
        | HirTy::AliasUse { entity, args, span } => {
            out.push((*entity, span.clone()));
            args.iter().for_each(|a| collect_hir_ty_entities(a, out));
        },
        HirTy::Param(entity, span) | HirTy::SelfType(entity, span) => {
            out.push((*entity, span.clone()));
        },
        HirTy::AssocProjection { base, assoc, span } => {
            collect_hir_ty_entities(base, out);
            out.push((*assoc, span.clone()));
        },
        HirTy::Tuple(elems, _) => elems.iter().for_each(|e| collect_hir_ty_entities(e, out)),
        HirTy::Function { params, ret, .. } => {
            params.iter().for_each(|p| collect_hir_ty_entities(p, out));
            collect_hir_ty_entities(ret, out);
        },
        HirTy::Opaque { bounds, .. } => bounds.iter().for_each(|b| collect_hir_ty_entities(b, out)),
        HirTy::Ref { inner, .. } => collect_hir_ty_entities(inner, out),
        HirTy::Never(_) | HirTy::Infer(_) | HirTy::Error(_) => {},
    }
}

/// The entity named at `offset` inside a type: the smallest named node whose
/// span contains it, so the cursor on `B` in `B.Item` is `B` and the cursor
/// on `Item` is the associated type.
pub fn hir_ty_entity_at(ty: &HirTy, offset: usize) -> Option<(Entity, Span)> {
    hir_ty_named_entities(ty)
        .into_iter()
        .filter(|(_, s)| s.start <= offset && offset <= s.end)
        .min_by_key(|(_, s)| s.end - s.start)
}

/// `body`'s HIR with its source map.
pub fn lowered_body(world: &World, root: Entity, body: Entity) -> Option<Arc<LoweredBody>> {
    world
        .query_context()
        .query(LowerBodyWithSourceMap { entity: body, root })
}

/// The HIR expression at `offset` in `body` (whose lowering is `lowered`):
/// the path segment there, else the innermost lowered node around it.
pub fn expr_at(
    world: &World,
    body: Entity,
    lowered: &LoweredBody,
    offset: usize,
) -> Option<HirExprId> {
    let root = kestrel_ast_builder::syntax::file_root(world, body)?;
    lowered
        .source_map
        .expr_at(&root, TextSize::from(offset as u32))
}

/// The HIR pattern at `offset` in `body`.
pub fn pat_at(
    world: &World,
    body: Entity,
    lowered: &LoweredBody,
    offset: usize,
) -> Option<HirPatId> {
    let root = kestrel_ast_builder::syntax::file_root(world, body)?;
    lowered
        .source_map
        .pat_at(&root, TextSize::from(offset as u32))
}

/// The local whose declaring identifier is at `offset` — a binding in a
/// body, or a parameter in a signature — with the body that owns it.
pub fn local_declared_at(
    world: &World,
    root: Entity,
    file_entity: Entity,
    offset: usize,
) -> Option<(Entity, LocalId)> {
    let pos = TextSize::from(offset as u32);
    // A binding inside a body, else a parameter in the signature of the
    // declaration around the cursor (its body's map records it).
    let candidates = [
        body_entity_at(world, file_entity, offset),
        enclosing_decl_at(world, file_entity, offset).filter(|d| world.has::<Valued>(*d)),
    ];
    candidates.into_iter().flatten().find_map(|body| {
        let local = lowered_body(world, root, body)?
            .source_map
            .local_declared_at(pos)?;
        Some((body, local))
    })
}

/// Where `local` (of `body`) is declared: its file and the span of its name.
/// `None` for a local the source does not spell (`self`, an implicit `it`,
/// a desugaring temporary).
pub fn local_name_site(
    world: &World,
    root: Entity,
    body: Entity,
    local: LocalId,
) -> Option<(Entity, Span)> {
    let lowered = lowered_body(world, root, body)?;
    let name = lowered.source_map.local_source(local)?.name;
    let file = crate::references::entity_file(world, body)?;
    Some((file, Span::new(file.index(), name.into())))
}

/// What the cursor names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A declaration — references span the workspace.
    Entity(Entity),
    /// A local of `body` — references stay in that body.
    Local { body: Entity, id: LocalId },
}

/// The symbol at `offset`: the one lookup behind rename, find-references and
/// document-highlight. In order:
///
/// 1. a type written in a signature or annotation (`Foo` in `x: Foo`);
/// 2. a local's declaring identifier (a binding, or a parameter name);
/// 3. the expression at the cursor in a body — a use of a local, a
///    definition, a member resolved by inference, a segment of a type in
///    expression position;
/// 4. a declaration's own name.
///
/// An overload set is ambiguous and resolves to nothing.
pub fn target_at(
    world: &World,
    root: Entity,
    file_entity: Entity,
    offset: usize,
) -> Option<Target> {
    let file_cst = kestrel_ast_builder::syntax::file_root(world, file_entity)?;
    if let Some((entity, _)) =
        crate::types::type_at_cursor(world, root, &file_cst, file_entity, offset)
    {
        return Some(Target::Entity(entity));
    }
    if let Some((body, id)) = local_declared_at(world, root, file_entity, offset) {
        return Some(Target::Local { body, id });
    }
    if let Some(body) = body_entity_at(world, file_entity, offset)
        && let Some(lowered) = lowered_body(world, root, body)
        && let Some(expr) = expr_at(world, body, &lowered, offset)
        && let Some(target) = resolve_expr(
            &world.query_context(),
            root,
            body,
            &lowered.body,
            expr,
            offset,
        )
    {
        return Some(target);
    }
    crate::references::decl_at_name_offset(world, file_entity, offset).map(Target::Entity)
}

/// What the expression `expr` of `body` names.
pub fn resolve_expr(
    ctx: &QueryContext<'_>,
    root: Entity,
    body: Entity,
    hir: &HirBody,
    expr: HirExprId,
    offset: usize,
) -> Option<Target> {
    match &hir.exprs[expr] {
        HirExpr::Def(entity, _, _) => Some(Target::Entity(*entity)),
        HirExpr::Local(id, _) => Some(Target::Local { body, id: *id }),
        // A type in expression position: the segment under the cursor.
        HirExpr::TypeRef { ty, .. } => hir_ty_entity_at(ty, offset).map(|(e, _)| Target::Entity(e)),
        // Ambiguous — which overload?
        HirExpr::OverloadSet { .. } => None,
        HirExpr::MethodCall { .. }
        | HirExpr::Field { .. }
        | HirExpr::Call { .. }
        | HirExpr::ImplicitMember { .. }
        | HirExpr::ProtocolCall { .. } => {
            let typed = ctx.query(InferBody { entity: body, root })?;
            typed.resolutions.get(&expr).copied().map(Target::Entity)
        },
        _ => None,
    }
}

pub fn hir_pat_span(pat: &HirPat) -> Span {
    match pat {
        HirPat::Wildcard { span }
        | HirPat::Binding { span, .. }
        | HirPat::Tuple { span, .. }
        | HirPat::Literal { span, .. }
        | HirPat::Range { span, .. }
        | HirPat::Variant { span, .. }
        | HirPat::ImplicitVariant { span, .. }
        | HirPat::Struct { span, .. }
        | HirPat::Array { span, .. }
        | HirPat::Or { span, .. }
        | HirPat::At { span, .. }
        | HirPat::Error { span } => span.clone(),
    }
}

/// Pull the CST root for a file entity from the compiler. We re-parse rather
/// than chasing `Valued` because not every node carries a CstNode pointer.
pub fn file_cst(compiler: &kestrel_compiler::Compiler, file_entity: Entity) -> SyntaxNode {
    compiler.parse(file_entity).tree()
}

/// The innermost declaration around `offset` in `file_entity`: walk out from
/// the token at the cursor to the first node a declaration's `CstNode`
/// points at. Falls back to the module that owns the file (the cursor is at
/// file scope, between declarations). Used for the lexical scope at the
/// cursor (completion, type lookups, rename collision checks) and for "the
/// cursor is on a declaration".
pub fn enclosing_decl_at(world: &World, file_entity: Entity, offset: usize) -> Option<Entity> {
    let decls: HashMap<SyntaxNodePtr, Entity> = world
        .iter_component::<CstNode>()
        .filter(|(e, _)| world.get::<FileId>(*e).is_some_and(|f| f.0 == file_entity))
        .map(|(e, cst)| (cst.0, e))
        .collect();
    // A declaration's node also holds the trivia (and attributes) before it;
    // the cursor is in the declaration from its `DeclSpan` on.
    let starts_by = |decl: Entity| {
        world
            .get::<kestrel_ast_builder::DeclSpan>(decl)
            .is_none_or(|s| s.0.start <= offset)
    };
    let innermost = kestrel_ast_builder::syntax::file_root(world, file_entity).and_then(|root| {
        let token = kestrel_hir_lower::source_map::token_at(&root, TextSize::from(offset as u32))?;
        token
            .parent_ancestors()
            .filter_map(|n| decls.get(&SyntaxNodePtr::new(&n)).copied())
            .find(|&decl| starts_by(decl))
    });
    innermost.or_else(|| {
        // Fall back to the module entity that owns this file. We find it by
        // looking at any other declaration's parent; that's the file's
        // module container. If the file is empty, return None.
        for (entity, fid) in world.iter_component::<FileId>() {
            if fid.0 != file_entity {
                continue;
            }
            // Walk up to find a Module ancestor.
            let mut cur = world.parent_of(entity);
            while let Some(e) = cur {
                if world.get::<NodeKind>(e) == Some(&NodeKind::Module) {
                    return Some(e);
                }
                cur = world.parent_of(e);
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    #[test]
    fn body_entity_at_finds_user_function() {
        let mut c = Compiler::new();
        // Note: Kestrel statements need explicit `;` terminators.
        let src = "module Test\nfunc foo() -> lang.i64 { 42 }\n";
        let f = c.set_source("/tmp/x.ks", src.into());
        c.build(f);
        // The body opens at the `{` and closes at the `}` — pick an offset
        // somewhere inside.
        let brace_open = src.find('{').expect("source has body");
        let body = body_entity_at(c.world(), f, brace_open + 2);
        assert!(body.is_some(), "expected body entity at body offset");
    }

    #[test]
    fn expr_at_returns_inner_expr() {
        let mut c = Compiler::new();
        let src = "module Test\nfunc foo() -> lang.i64 { 42 }\n";
        let f = c.set_source("/tmp/x.ks", src.into());
        c.build(f);

        let body_offset = src.find("42").unwrap();
        let body_entity = body_entity_at(c.world(), f, body_offset).expect("body");
        let world = c.world();
        let lowered = lowered_body(world, c.root(), body_entity).expect("hir");

        let id = expr_at(world, body_entity, &lowered, body_offset).expect("expr");
        let span = hir_expr_span(&lowered.body.exprs[id]);
        assert_eq!(&src[span.start..span.end], "42");
    }

    /// The declaration around a cursor comes from the `CstNode` pointers:
    /// a method inside a struct, the struct between its members, the
    /// module at file scope.
    #[test]
    fn enclosing_decl_at_walks_out_to_the_innermost_declaration() {
        use kestrel_ast_builder::Name;
        let mut c = Compiler::new();
        let src =
            "module Test\n\nstruct S {\n  var a: lang.i64\n\n  func m() -> lang.i64 { 1 }\n}\n";
        let f = c.set_source("/tmp/decl_at.ks", src.into());
        c.build(f);
        let world = c.world();
        let name_at = |offset: usize| {
            let decl = enclosing_decl_at(world, f, offset).expect("decl");
            (
                world.get::<NodeKind>(decl).cloned(),
                world.get::<Name>(decl).map(|n| n.0.clone()),
            )
        };
        assert_eq!(
            name_at(src.find("1 }").unwrap()),
            (Some(NodeKind::Function), Some("m".into()))
        );
        assert_eq!(
            name_at(src.find("\n\n  func").unwrap() + 1),
            (Some(NodeKind::Struct), Some("S".into()))
        );
        assert_eq!(
            name_at(src.find("\n\nstruct").unwrap() + 1).0,
            Some(NodeKind::Module)
        );
    }

    /// G26: `B.Item.zero()` lowers its receiver to `HirExpr::TypeRef`; the
    /// walker must name `B` under the cursor on `B` and the associated type
    /// `Item` under the cursor on `Item` — not the whole path as one entity.
    #[test]
    fn hir_ty_entity_at_splits_projection_segments() {
        use kestrel_ast_builder::Name;
        let mut c = Compiler::new();
        let src = "module T\n\
                   protocol Zero { static func zero() -> Self }\n\
                   protocol Producer { type Item; func produce() -> Item }\n\
                   func make[B](b: B) -> B.Item where B: Producer, B.Item: Zero { B.Item.zero() }\n";
        let f = c.set_source("/tmp/typeref.ks", src.into());
        c.build(f);
        let at = src.find("B.Item.zero").unwrap();
        let body = body_entity_at(c.world(), f, at).expect("body");
        let world = c.world();
        let ctx = world.query_context();
        let hir = ctx
            .query(kestrel_hir_lower::LowerBody {
                entity: body,
                root: c.root(),
            })
            .expect("hir");
        let ty = hir
            .exprs
            .iter()
            .find_map(|(_, e)| match e {
                HirExpr::TypeRef { ty, .. } => Some(ty.clone()),
                _ => None,
            })
            .expect("receiver `B.Item` lowers to a TypeRef");
        let name_at = |offset: usize| {
            let (e, span) = hir_ty_entity_at(&ty, offset).expect("entity at offset");
            (
                world.get::<Name>(e).map(|n| n.0.clone()),
                &src[span.start..span.end],
            )
        };
        assert_eq!(name_at(at), (Some("B".into()), "B"));
        let (item, text) = name_at(at + "B.".len());
        assert_eq!(item, Some("Item".into()));
        assert!(
            text.ends_with("Item"),
            "span should end at `Item`: {text:?}"
        );
    }
}
