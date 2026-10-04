//! Bridge between cursor positions and compiler entities / HIR nodes.
//!
//! Most of M2's "what's at the cursor?" logic funnels through these helpers
//! so handlers stay narrow and the lookup rules stay in one place.

use kestrel_ast_builder::{DeclSpan, FileId, FilePath, NodeKind, Valued};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId, HirPat, HirPatId};
use kestrel_hir::ty::HirTy;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxNode;
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

/// Find the smallest HIR expression whose span contains `offset`. Returns
/// `None` when no expression covers the offset (e.g. cursor in trivia
/// between expressions, or in the function signature).
pub fn hir_expr_at(body: &HirBody, offset: usize) -> Option<HirExprId> {
    let mut best: Option<(HirExprId, usize)> = None;
    for (id, expr) in body.exprs.iter() {
        let span = hir_expr_span(expr);
        if span.start <= offset && offset <= span.end {
            let len = span.end - span.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((id, len));
            }
        }
    }
    best.map(|(id, _)| id)
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

/// Find the smallest HIR pattern whose span contains `offset`.
pub fn hir_pat_at(body: &HirBody, offset: usize) -> Option<HirPatId> {
    let mut best: Option<(HirPatId, usize)> = None;
    for (id, pat) in body.pats.iter() {
        let span = hir_pat_span(pat);
        if span.start <= offset && offset <= span.end {
            let len = span.end - span.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((id, len));
            }
        }
    }
    best.map(|(id, _)| id)
}

/// Pull the CST root for a file entity from the compiler. We re-parse rather
/// than chasing `Valued` because not every node carries a CstNode pointer.
pub fn file_cst(compiler: &kestrel_compiler::Compiler, file_entity: Entity) -> SyntaxNode {
    compiler.parse(file_entity).tree()
}

/// Smallest entity in `file_entity` whose `DeclSpan` covers `offset`. Falls
/// back to walking the module hierarchy when no `DeclSpan` matches (e.g.
/// the cursor is at file scope between two top-level decls). Used by
/// completion to find the lexical scope at the cursor.
pub fn enclosing_decl_at(world: &World, file_entity: Entity, offset: usize) -> Option<Entity> {
    let mut best: Option<(Entity, usize)> = None;
    for (entity, span) in world.iter_component::<DeclSpan>() {
        let Some(fid) = world.get::<FileId>(entity) else {
            continue;
        };
        if fid.0 != file_entity {
            continue;
        }
        let s = &span.0;
        if s.start <= offset && offset <= s.end {
            let len = s.end - s.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((entity, len));
            }
        }
    }
    best.map(|(e, _)| e).or_else(|| {
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
    fn hir_expr_at_returns_inner_expr() {
        let mut c = Compiler::new();
        let src = "module Test\nfunc foo() -> lang.i64 { 42 }\n";
        let f = c.set_source("/tmp/x.ks", src.into());
        c.build(f);

        let body_offset = src.find("42").unwrap();
        let body_entity = body_entity_at(c.world(), f, body_offset).expect("body");
        let world = c.world();
        let ctx = world.query_context();
        let hir = ctx
            .query(kestrel_hir_lower::LowerBody {
                entity: body_entity,
                root: c.root(),
            })
            .expect("hir");

        let id = hir_expr_at(&hir, body_offset).expect("expr");
        let span = hir_expr_span(&hir.exprs[id]);
        assert_eq!(&src[span.start..span.end], "42");
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
