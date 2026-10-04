//! Inverse of go-to-def: given a target entity (or a local), enumerate every
//! site in the workspace that resolves to it.
//!
//! Lives in the LSP crate rather than `kestrel-name-res` because the walk
//! needs both `LowerBody` (from `kestrel-hir-lower`) and `InferBody` (from
//! `kestrel-type-infer`), and `kestrel-name-res` sits below both in the
//! dependency graph. The LSP rebuilds the compiler per request, so cross-call
//! memoisation isn't load-bearing yet — this is a free function, not a
//! `QueryFn`.
//!
//! Coverage today:
//! - Direct entity references in expressions: `HirExpr::Def(target, ...)`,
//!   `OverloadSet` containing `target`, `ProtocolCall { protocol: target }`,
//!   and every segment of a `HirExpr::TypeRef` that names `target`.
//! - Pattern-position references: `HirPat::Variant`, `HirPat::Struct`.
//! - Inference-resolved member accesses: anything in `TypedBody::resolutions`
//!   that maps to `target` (covers `Field`, `MethodCall`, `Call`,
//!   `ImplicitMember`, `ProtocolCall::method`).
//! - Local-variable uses (within one body).
//!
//! Not yet covered:
//! - Type-position references (`Foo` in `func bar(x: Foo)`). These are
//!   resolved during AST build via `ResolveTypePath` and the resolved
//!   entity isn't kept on the AST node. Add a `ResolveTypeRefs { file }`
//!   query in `kestrel-name-res` when needed.

use kestrel_ast_builder::{Body, DeclSpan, FileId};
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirExpr, HirPat};
use kestrel_hir::res::LocalId;
use kestrel_hir_lower::LowerBody;
use kestrel_span::Span;
use kestrel_syntax_tree::utils::get_name_span;
use kestrel_type_infer::InferBody;

use crate::semantic::hir_expr_span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RefKind {
    /// Direct entity reference: span IS the identifier (`foo`, `Bar`, etc).
    Direct,
    /// Member access resolved by type inference. Span covers the whole
    /// expression (`obj.field`, `obj.method(...)`); callers clip to the
    /// trailing identifier when they need just the name.
    MemberAccess,
    /// Pattern position (`Variant`, `Struct`). Span covers the whole
    /// pattern; callers clip to the path identifier when needed.
    Pattern,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReferenceSite {
    pub file: Entity,
    pub span: Span,
    pub kind: RefKind,
}

/// Find every reference to `target` in the workspace.
pub fn references_to(world: &World, root: Entity, target: Entity) -> Vec<ReferenceSite> {
    let ctx = world.query_context();
    let body_entities: Vec<Entity> = world.iter_component::<Body>().map(|(e, _)| e).collect();

    let mut sites: Vec<ReferenceSite> = Vec::new();

    for body_entity in body_entities {
        let Some(file) = entity_file(world, body_entity) else {
            continue;
        };
        let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) else {
            continue;
        };

        for (_, expr) in hir.exprs.iter() {
            match expr {
                HirExpr::Def(e, _, span) if *e == target => sites.push(ReferenceSite {
                    file,
                    span: span.clone(),
                    kind: RefKind::Direct,
                }),
                HirExpr::OverloadSet {
                    candidates, span, ..
                } if candidates.contains(&target) => sites.push(ReferenceSite {
                    file,
                    span: span.clone(),
                    kind: RefKind::Direct,
                }),
                HirExpr::ProtocolCall { protocol, span, .. } if *protocol == target => {
                    sites.push(ReferenceSite {
                        file,
                        span: span.clone(),
                        kind: RefKind::Direct,
                    })
                },
                // A type in expression position (`B.Item`): each segment that
                // names `target`. A projection's span covers its base, so the
                // site is clipped to the trailing identifier like a member.
                HirExpr::TypeRef { ty, .. } => sites.extend(
                    crate::semantic::hir_ty_named_entities(ty)
                        .into_iter()
                        .filter(|(e, _)| *e == target)
                        .map(|(_, span)| ReferenceSite {
                            file,
                            span,
                            kind: RefKind::MemberAccess,
                        }),
                ),
                _ => {},
            }
        }

        for (_, pat) in hir.pats.iter() {
            match pat {
                HirPat::Variant { entity, span, .. } if *entity == target => {
                    sites.push(ReferenceSite {
                        file,
                        span: span.clone(),
                        kind: RefKind::Pattern,
                    })
                },
                HirPat::Struct { entity, span, .. } if *entity == target => {
                    sites.push(ReferenceSite {
                        file,
                        span: span.clone(),
                        kind: RefKind::Pattern,
                    })
                },
                _ => {},
            }
        }

        let Some(typed) = ctx.query(InferBody {
            entity: body_entity,
            root,
        }) else {
            continue;
        };
        for (&expr_id, &resolved) in typed.resolutions.iter() {
            if resolved != target {
                continue;
            }
            let span = hir_expr_span(&hir.exprs[expr_id]);
            sites.push(ReferenceSite {
                file,
                span,
                kind: RefKind::MemberAccess,
            });
        }
    }

    sites.sort_by_key(sort_key);
    sites.dedup();
    sites
}

fn sort_key(s: &ReferenceSite) -> (usize, usize, usize, u8) {
    let kind = match s.kind {
        RefKind::Direct => 0,
        RefKind::MemberAccess => 1,
        RefKind::Pattern => 2,
    };
    (s.file.index(), s.span.start, s.span.end, kind)
}

/// Find references to a local within its owning body. Locals don't escape, so
/// this never crosses body boundaries.
pub fn local_references(
    world: &World,
    body_entity: Entity,
    root: Entity,
    local: LocalId,
) -> Vec<ReferenceSite> {
    let ctx = world.query_context();
    let Some(file) = entity_file(world, body_entity) else {
        return Vec::new();
    };
    let Some(hir) = ctx.query(LowerBody {
        entity: body_entity,
        root,
    }) else {
        return Vec::new();
    };

    let mut sites: Vec<ReferenceSite> = Vec::new();
    for (_, expr) in hir.exprs.iter() {
        if let HirExpr::Local(id, span) = expr
            && *id == local
        {
            sites.push(ReferenceSite {
                file,
                span: span.clone(),
                kind: RefKind::Direct,
            });
        }
    }
    sites.sort_by_key(sort_key);
    sites.dedup();
    sites
}

/// For `MemberAccess` and `Pattern` spans, find the trailing identifier
/// substring and return its sub-span. For `Direct`, the span is already the
/// identifier.
pub fn clip_to_identifier(source: &str, span: &Span, kind: RefKind) -> Span {
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

pub fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

/// Does `span` literally spell `name` in `source`?
///
/// The single fail-closed predicate for "is this span actually the identifier
/// it claims to name?". Renaming rewrites the bytes under a span, so any span
/// that does not spell its own name will corrupt source when edited. Today the
/// offenders are all `HirBody::locals`:
/// - a `let`/`var` binding's `Local::span` is the whole *statement*
///   (`kestrel-hir-lower/src/stmt.rs:126`),
/// - a parameter's and `self`'s is `Span::synthetic(0)` = `0..0`
///   (`kestrel-hir-lower/src/lib.rs:71,79`),
/// - every desugaring temp (`$iter`, `$try_value`, `$dsi`, `$opts`,
///   `$let_tmp`, `_cparam_N`) borrows the span of the construct it came from.
///
/// Deliberately *not* a `is_synthetic() || name.starts_with('$')` heuristic:
/// closure-destructure params (`_cparam_N`,
/// `kestrel-hir-lower/src/expr.rs:1445`) get a real, non-synthetic span (the
/// closure's) and a name with no `$`, so both heuristics wave them through.
/// Text equality is the property we actually need, and it stays correct for
/// spans and synthetic names that don't exist yet.
pub fn span_spells_name(source: &str, span: &Span, name: &str) -> bool {
    source.get(span.start..span.end) == Some(name)
}

/// The `enclosing_decl_at` fallback, restricted to the decl's *own name*.
///
/// `semantic::enclosing_decl_at` resolves any offset inside a declaration's
/// extent to that declaration — including every offset in its body. Handlers
/// that use it as the "cursor is on a declaration's identifier" fallback must
/// therefore re-check that the cursor really is on the identifier: otherwise a
/// cursor on a `let` binding or a parameter name (neither of which is an
/// `HirExpr`, so `hir_expr_at` can't see them) silently resolves to the
/// enclosing function — and rename then renames *that* workspace-wide.
///
/// `get_name_span` is the same function `rename::identifier_for_target` uses
/// to find the text it would edit, so the two agree by construction.
pub fn decl_at_name_offset(world: &World, file_entity: Entity, offset: usize) -> Option<Entity> {
    let decl = crate::semantic::enclosing_decl_at(world, file_entity, offset)?;
    let cst = kestrel_ast_builder::syntax::cst_node(world, decl)?;
    let decl_span = world.get::<DeclSpan>(decl)?;
    let name_span = get_name_span(&cst, decl_span.0.file_id)?;
    (name_span.start <= offset && offset <= name_span.end).then_some(decl)
}

pub fn entity_file(world: &World, entity: Entity) -> Option<Entity> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    /// Find the entity for a top-level decl named `name` in `file`.
    fn find_decl(world: &World, file: Entity, name: &str) -> Entity {
        use kestrel_ast_builder::{FileId as F, Name};
        for (e, n) in world.iter_component::<Name>() {
            if n.0 != name {
                continue;
            }
            if let Some(fid) = world.get::<F>(e)
                && fid.0 == file
            {
                return e;
            }
        }
        panic!("decl `{name}` not found in file");
    }

    #[test]
    fn references_to_function_picks_up_calls() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func target() -> lang.i64 { 1 }\n\
                   func caller() -> lang.i64 { target() }\n";
        let f = c.set_source("/tmp/refs.ks", src.into());
        c.build(f);

        let target = find_decl(c.world(), f, "target");
        let refs = references_to(c.world(), c.root(), target);

        let texts: Vec<&str> = refs
            .iter()
            .map(|r| &src[r.span.start..r.span.end])
            .collect();
        assert!(
            texts.contains(&"target"),
            "expected `target` reference; got {texts:?}"
        );
    }

    #[test]
    fn references_to_function_excludes_unrelated_names() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func a() -> lang.i64 { 1 }\n\
                   func b() -> lang.i64 { 2 }\n\
                   func c() -> lang.i64 { a() }\n";
        let f = c.set_source("/tmp/refs2.ks", src.into());
        c.build(f);

        let a = find_decl(c.world(), f, "a");
        let refs = references_to(c.world(), c.root(), a);
        assert_eq!(
            refs.len(),
            1,
            "expected exactly one reference to `a`, got {refs:?}"
        );
    }

    #[test]
    fn local_references_finds_within_body() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   func foo() -> lang.i64 {\n  \
                     let x = 1;\n  \
                     let y = x + x;\n  \
                     y\n\
                   }\n";
        let f = c.set_source("/tmp/local_refs.ks", src.into());
        c.build(f);

        let foo = find_decl(c.world(), f, "foo");
        let world = c.world();
        let ctx = world.query_context();
        let hir = ctx
            .query(LowerBody {
                entity: foo,
                root: c.root(),
            })
            .expect("hir");

        // Locate local `x` by name.
        let x_id = hir
            .locals
            .iter()
            .find(|(_, l)| l.name == "x")
            .map(|(id, _)| id)
            .expect("local x");

        let refs = local_references(world, foo, c.root(), x_id);
        // `x + x` lowers to a protocol call with two `Local(x)` operands. Two refs.
        assert_eq!(refs.len(), 2, "expected 2 refs to x, got {refs:?}");
    }
}
