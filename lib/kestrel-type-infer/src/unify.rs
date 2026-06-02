//! Unification with literal guards, Error/Never handling.
//!
//! The core operation: make two TyVars equal by binding one to the other
//! or recursively unifying their structure. Special handling for:
//! - Error types: absorb silently (prevent cascading errors)
//! - Never types: unify with anything (bottom type)
//! - Literal markers: guard against unification with non-conforming types

use crate::ctx::InferCtx;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use kestrel_ast_builder::{Intrinsic, Name, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_name_res::expand_protocol_closure;

/// Whether two associated-type entities denote the same projected type for
/// unification. True when they're the same entity, or when they share a name
/// AND their parent protocols are refinement-related (one transitively refines
/// the other). The name alone is *not* sufficient: unrelated protocols can each
/// declare a same-named assoc type (`Foo.Item` vs `Bar.Item`) bound to
/// different concrete types, and collapsing those would silently mask a type
/// error. The refinement gate admits the genuine case — `Iterator.Item` ≡
/// `Iterable.Item` via the blanket `extend Iterator: Iterable` (so
/// `T.TargetIterator.Item` unifies with bare `T.Item`) — while rejecting
/// coincidental name clashes. Single source of truth for both the
/// TypeAlias/TypeAlias and AssocProjection/TypeAlias arms below.
fn assoc_entities_unify(ctx: &InferCtx<'_>, a: Entity, b: Entity) -> bool {
    if a == b {
        return true;
    }
    let (Some(na), Some(nb)) = (ctx.query_ctx.get::<Name>(a), ctx.query_ctx.get::<Name>(b)) else {
        return false;
    };
    if na.0 != nb.0 {
        return false;
    }
    let (Some(pa), Some(pb)) = (ctx.query_ctx.parent_of(a), ctx.query_ctx.parent_of(b)) else {
        return false;
    };
    expand_protocol_closure(ctx.query_ctx, ctx.root, [pa]).contains(&pb)
        || expand_protocol_closure(ctx.query_ctx, ctx.root, [pb]).contains(&pa)
}

/// Unification failure reason.
#[derive(Debug)]
pub enum UnifyError {
    /// Structural type mismatch (different entities, arities, etc.)
    Mismatch,
    /// Literal TyVar couldn't adopt target type (doesn't conform to ExpressibleBy*).
    LiteralGuard,
    /// Occurs check: TyVar appears in its own type (infinite type).
    OccursCheck,
}

/// Unify two type variables, making them equivalent.
pub fn unify(ctx: &mut InferCtx<'_>, a: TyVar, b: TyVar) -> Result<(), UnifyError> {
    let a = ctx.resolve(a);
    let b = ctx.resolve(b);

    // Same TyVar — trivially equal
    if a == b {
        return Ok(());
    }

    // We need to read both slots, but we can't hold two &-refs while mutating.
    // Clone the slots to avoid borrow issues.
    let slot_a = ctx.types[a.0 as usize].clone();
    let slot_b = ctx.types[b.0 as usize].clone();

    match (&slot_a, &slot_b) {
        // Error poisons: silently absorb
        (TySlot::Resolved(TyKind::Error), _) | (_, TySlot::Resolved(TyKind::Error)) => Ok(()),

        // Never (bottom type): unifies with anything.
        // Never + Unresolved: don't bind — a sibling constraint may yet pin
        // the slot to a concrete type (e.g. one match arm is `return`, the
        // other is `Int`; the result should be `Int`). Instead, record the
        // Unresolved side so the post-fixpoint never-fallback (mirror of
        // Rust's `never_type_fallback`) can default it to Never if nothing
        // else ever constrained it.
        (TySlot::Resolved(TyKind::Never), TySlot::Unresolved { .. }) => {
            ctx.never_fallback_targets.insert(b);
            Ok(())
        },
        (TySlot::Unresolved { .. }, TySlot::Resolved(TyKind::Never)) => {
            ctx.never_fallback_targets.insert(a);
            Ok(())
        },
        (TySlot::Resolved(TyKind::Never), _) | (_, TySlot::Resolved(TyKind::Never)) => Ok(()),

        // Both unresolved: link them, merge literal markers.
        // If both have different literal kinds, that's a mismatch
        // (e.g., integer literal vs string literal in if/else branches).
        (TySlot::Unresolved { literal: lit_a }, TySlot::Unresolved { literal: lit_b }) => {
            if let (Some(a_kind), Some(b_kind)) = (lit_a, lit_b)
                && a_kind != b_kind
            {
                return Err(UnifyError::Mismatch);
            }
            // Propagate wildcard status: if either side is a wildcard, the root
            // (b, since a redirects to b) must also be a wildcard so that
            // report_unresolved_slots skips it.
            if ctx.wildcard_tvars.contains(&a) || ctx.wildcard_tvars.contains(&b) {
                ctx.wildcard_tvars.insert(a);
                ctx.wildcard_tvars.insert(b);
            }
            let merged = lit_a.or(*lit_b);
            ctx.types[a.0 as usize] = TySlot::Redirect(b);
            if merged.is_some() {
                ctx.types[b.0 as usize] = TySlot::Unresolved { literal: merged };
            }
            Ok(())
        },

        // Unresolved (non-literal) + Concrete: bind
        (TySlot::Unresolved { literal: None }, _) => {
            // Wildcard unifying with a concrete type: wildcard resolves normally,
            // no propagation needed (it becomes concrete, not Unresolved).
            occurs_check(ctx, a, b)?;
            ctx.types[a.0 as usize] = TySlot::Redirect(b);
            Ok(())
        },
        (_, TySlot::Unresolved { literal: None }) => {
            occurs_check(ctx, b, a)?;
            ctx.types[b.0 as usize] = TySlot::Redirect(a);
            Ok(())
        },

        // Literal TyVar + Concrete: guard with ExpressibleBy* conformance
        (TySlot::Unresolved { literal: Some(lit) }, TySlot::Resolved(kind)) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, a, b)?;
                ctx.types[a.0 as usize] = TySlot::Redirect(b);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        },
        (TySlot::Resolved(kind), TySlot::Unresolved { literal: Some(lit) }) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, b, a)?;
                ctx.types[b.0 as usize] = TySlot::Redirect(a);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        },

        // Both concrete: structural unification
        (TySlot::Resolved(kind_a), TySlot::Resolved(kind_b)) => unify_concrete(ctx, kind_a, kind_b),

        // Redirect should be resolved by resolve()
        _ => unreachable!("resolve() should have followed redirects"),
    }
}

/// Structural unification of two concrete types.
fn unify_concrete(ctx: &mut InferCtx<'_>, a: &TyKind, b: &TyKind) -> Result<(), UnifyError> {
    match (a, b) {
        // Nominal types: same entity + unify type args pairwise. Each nominal
        // category only unifies with itself (Struct with Struct, Enum with Enum, etc.).
        (
            TyKind::Struct {
                entity: ea,
                args: aa,
            },
            TyKind::Struct {
                entity: eb,
                args: ab,
            },
        )
        | (
            TyKind::Enum {
                entity: ea,
                args: aa,
            },
            TyKind::Enum {
                entity: eb,
                args: ab,
            },
        )
        | (
            TyKind::Protocol {
                entity: ea,
                args: aa,
            },
            TyKind::Protocol {
                entity: eb,
                args: ab,
            },
        ) => {
            if ea != eb || aa.len() != ab.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> = aa.iter().copied().zip(ab.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            Ok(())
        },

        // TypeAlias vs TypeAlias: same entity, or same-named assoc types from
        // refinement-related protocols (e.g. Iterator.Item ≡ Iterable.Item) —
        // see `assoc_entities_unify`. Require same arg count.
        (
            TyKind::TypeAlias {
                entity: ea,
                args: aa,
            },
            TyKind::TypeAlias {
                entity: eb,
                args: ab,
            },
        ) => {
            if !assoc_entities_unify(ctx, *ea, *eb) {
                return Err(UnifyError::Mismatch);
            }
            if aa.len() != ab.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> = aa.iter().copied().zip(ab.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            Ok(())
        },

        // Tuples: same arity + unify elements pairwise
        (TyKind::Tuple(ea), TyKind::Tuple(eb)) => {
            if ea.len() != eb.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> = ea.iter().copied().zip(eb.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            Ok(())
        },

        // Functions: same arity + unify params + unify return
        (
            TyKind::Function {
                params: pa,
                ret: ra,
            },
            TyKind::Function {
                params: pb,
                ret: rb,
            },
        ) => {
            if pa.len() != pb.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> = pa.iter().copied().zip(pb.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            unify(ctx, *ra, *rb)
        },

        // Type params: must be the same entity
        (TyKind::Param { entity: a }, TyKind::Param { entity: b }) => {
            if a == b {
                Ok(())
            } else {
                Err(UnifyError::Mismatch)
            }
        },

        // SelfType: must be the same protocol entity. SelfType(P) ~ SelfType(P)
        // succeeds; otherwise mismatch. (Unification with concrete types is
        // handled at the call-site boundary by `lower_hir_ty_sub`, which swaps
        // SelfType for `recv_tv` when the receiver is known.)
        (TyKind::SelfType { entity: a }, TyKind::SelfType { entity: b }) => {
            if a == b {
                Ok(())
            } else {
                Err(UnifyError::Mismatch)
            }
        },

        // SelfType(P) unifies with Protocol(P) (no args) — the abstract Self
        // of P is an instance of P. Used when a where-clause or signature
        // reference produces Protocol(P) on one side while Self lowered to
        // SelfType(P) on the other. No arg unification needed: SelfType has
        // no args, so Protocol side must have none too.
        (TyKind::SelfType { entity: a }, TyKind::Protocol { entity: b, args })
        | (TyKind::Protocol { entity: b, args }, TyKind::SelfType { entity: a }) => {
            if a == b && args.is_empty() {
                Ok(())
            } else {
                Err(UnifyError::Mismatch)
            }
        },

        // AssocProjections: same assoc entity + unified bases
        (
            TyKind::AssocProjection {
                base: ba,
                assoc: aa,
            },
            TyKind::AssocProjection {
                base: bb,
                assoc: ab,
            },
        ) => {
            if aa != ab {
                return Err(UnifyError::Mismatch);
            }
            unify(ctx, *ba, *bb)
        },

        // AssocProjection vs bare TypeAlias: the bare TypeAlias has already
        // dropped its base (e.g. a protocol method's return references the assoc
        // type bare), so the base can't be compared — fall back to matching the
        // assoc entities via `assoc_entities_unify` (same entity, or same-named
        // across refinement-related protocols). This is what lets
        // `T.TargetIterator.Item` (Iterator.Item) unify with bare `T.Item`
        // (Iterable.Item), since `extend Iterator: Iterable` relates them.
        (TyKind::AssocProjection { assoc: a, .. }, TyKind::TypeAlias { entity: e, .. })
        | (TyKind::TypeAlias { entity: e, .. }, TyKind::AssocProjection { assoc: a, .. })
            if assoc_entities_unify(ctx, *a, *e) =>
        {
            Ok(())
        },

        // Opaque types: same origin + same index → pairwise unify bound args
        (
            TyKind::Opaque {
                origin: oa,
                bounds: ba,
                index: ia,
                ..
            },
            TyKind::Opaque {
                origin: ob,
                bounds: bb,
                index: ib,
                ..
            },
        ) => {
            if oa != ob || ia != ib || ba.len() != bb.len() {
                return Err(UnifyError::Mismatch);
            }
            // Pairwise unify the protocol type args within each bound
            for (bound_a, bound_b) in ba.iter().zip(bb.iter()) {
                if bound_a.0 != bound_b.0 || bound_a.1.len() != bound_b.1.len() {
                    return Err(UnifyError::Mismatch);
                }
                let pairs: Vec<(TyVar, TyVar)> = bound_a
                    .1
                    .iter()
                    .copied()
                    .zip(bound_b.1.iter().copied())
                    .collect();
                for (a, b) in pairs {
                    unify(ctx, a, b)?;
                }
            }
            Ok(())
        },

        // Opaque vs any concrete type is a mismatch (opaque hides identity)
        (TyKind::Opaque { .. }, _) | (_, TyKind::Opaque { .. }) => Err(UnifyError::Mismatch),

        // Error already handled above; remaining combos are mismatches
        _ => Err(UnifyError::Mismatch),
    }
}

/// Occurs check: ensure `tv` doesn't appear in `target` (prevents infinite types).
fn occurs_check(ctx: &InferCtx<'_>, tv: TyVar, target: TyVar) -> Result<(), UnifyError> {
    let target = ctx.resolve(target);
    if tv == target {
        return Err(UnifyError::OccursCheck);
    }
    match &ctx.types[target.0 as usize] {
        TySlot::Resolved(TyKind::Struct { args, .. })
        | TySlot::Resolved(TyKind::Enum { args, .. })
        | TySlot::Resolved(TyKind::Protocol { args, .. })
        | TySlot::Resolved(TyKind::TypeAlias { args, .. }) => {
            for &arg in args {
                occurs_check(ctx, tv, arg)?;
            }
            Ok(())
        },
        TySlot::Resolved(TyKind::Tuple(elems)) => {
            for &e in elems {
                occurs_check(ctx, tv, e)?;
            }
            Ok(())
        },
        TySlot::Resolved(TyKind::Function { params, ret }) => {
            for &p in params {
                occurs_check(ctx, tv, p)?;
            }
            occurs_check(ctx, tv, *ret)
        },
        TySlot::Resolved(TyKind::AssocProjection { base, .. }) => occurs_check(ctx, tv, *base),
        TySlot::Resolved(TyKind::Opaque {
            bounds,
            origin_args,
            ..
        }) => {
            for (_, args) in bounds {
                for &arg in args {
                    occurs_check(ctx, tv, arg)?;
                }
            }
            for &arg in origin_args {
                occurs_check(ctx, tv, arg)?;
            }
            Ok(())
        },
        _ => Ok(()),
    }
}

/// Check if a concrete type conforms to the literal's ExpressibleBy* protocol.
pub fn conforms_to_literal_protocol(ctx: &InferCtx<'_>, ty: &TyKind, lit: LiteralKind) -> bool {
    // Special case: intrinsic types (lang.i32, lang.f64, etc.) don't have
    // protocol conformances but should accept matching literals directly.
    if is_intrinsic_literal_compatible(ctx, ty, lit) {
        return true;
    }

    let feature = match lit {
        LiteralKind::Integer => Builtin::ExpressibleByIntegerLiteral,
        LiteralKind::Float => Builtin::ExpressibleByFloatLiteral,
        LiteralKind::String => Builtin::ExpressibleByStringLiteral,
        LiteralKind::Bool => Builtin::ExpressibleByBoolLiteral,
        LiteralKind::Char => Builtin::ExpressibleByCharLiteral,
        LiteralKind::Null => Builtin::ExpressibleByNullLiteral,
        LiteralKind::Array => Builtin::InternalExpressibleByArrayLiteral,
        LiteralKind::Dictionary => Builtin::InternalExpressibleByDictionaryLiteral,
        // Accumulator type — InterpolationLink validates the type through
        // associated type resolution, so accept any concrete type here.
        LiteralKind::StringInterpolation => return true,
    };
    let Some(protocol) = ctx.resolver.builtin(feature) else {
        return false;
    };
    ctx.resolver.conforms_to(ty, protocol)
}

/// Intrinsic types accept matching literals without protocol conformance.
/// e.g. integer literals → i8/i16/i32/i64/u8/u16/u32/u64,
///      bool literals → i1, string literals → str
fn is_intrinsic_literal_compatible(ctx: &InferCtx<'_>, ty: &TyKind, lit: LiteralKind) -> bool {
    let TyKind::Struct { entity, .. } = ty else {
        return false;
    };
    // Must be an intrinsic struct
    if ctx.query_ctx.get::<Intrinsic>(*entity).is_none()
        || ctx.query_ctx.get::<NodeKind>(*entity) != Some(&NodeKind::Struct)
    {
        return false;
    }
    let Some(name) = ctx.query_ctx.get::<Name>(*entity) else {
        return false;
    };
    match lit {
        LiteralKind::Integer => matches!(name.0.as_str(), "i8" | "i16" | "i32" | "i64"),
        LiteralKind::Float => matches!(name.0.as_str(), "f32" | "f64"),
        LiteralKind::Bool => matches!(name.0.as_str(), "i1"),
        LiteralKind::String => matches!(name.0.as_str(), "str"),
        LiteralKind::Char => matches!(name.0.as_str(), "i32"),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::CallArg;
    use crate::resolve::*;
    use kestrel_hecs::Entity;

    /// Minimal mock resolver that doesn't know about any types.
    struct NullResolver;

    impl TypeResolver for NullResolver {
        fn resolve_member(
            &self,
            _: &TyKind,
            _: &str,
            _: &[CallArg],
        ) -> Result<MemberResolution, MemberError> {
            Err(MemberError::NotFound)
        }
        fn resolve_single_member(
            &self,
            _: &TyKind,
            _: Entity,
        ) -> Result<MemberResolution, MemberError> {
            Err(MemberError::NotFound)
        }
        fn conforms_to(&self, _: &TyKind, _: Entity) -> bool {
            false
        }
        fn resolve_associated_type(&self, _: &TyKind, _: &str) -> Option<AssociatedTypeResolution> {
            None
        }
        fn builtin(&self, _: Builtin) -> Option<Entity> {
            None
        }
        fn check_promotion(&self, _: &TyKind, _: &TyKind) -> Option<Entity> {
            None
        }
    }

    /// Dummy entity for tests (not backed by a real World).
    fn dummy() -> Entity {
        Entity::from_raw(999)
    }

    fn make_world() -> kestrel_hecs::World {
        let mut world = kestrel_hecs::World::new();
        world.begin_revision();
        world
    }

    fn make_ctx<'a>(
        resolver: &'a dyn TypeResolver,
        qctx: &'a kestrel_hecs::QueryContext<'a>,
    ) -> InferCtx<'a> {
        InferCtx::new(resolver, qctx, dummy(), dummy())
    }

    #[test]
    fn unify_same_var() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a = ctx.fresh();
        assert!(unify(&mut ctx, a, a).is_ok());
    }

    #[test]
    fn unify_two_unresolved() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a = ctx.fresh();
        let b = ctx.fresh();
        assert!(unify(&mut ctx, a, b).is_ok());
        assert_eq!(ctx.resolve(a), ctx.resolve(b));
    }

    #[test]
    fn unify_unresolved_with_concrete() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a = ctx.fresh();
        let b = ctx.named(dummy(), vec![]);
        assert!(unify(&mut ctx, a, b).is_ok());
        assert!(ctx.is_concrete(a));
    }

    #[test]
    fn unify_mismatch() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a = ctx.named(dummy(), vec![]);
        let b = ctx.tuple(vec![]);
        assert!(matches!(unify(&mut ctx, a, b), Err(UnifyError::Mismatch)));
    }

    #[test]
    fn unify_error_absorbs() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let err = ctx.report_error(crate::error::InferError::InfiniteType {
            span: kestrel_span::Span::synthetic(0),
        });
        let b = ctx.fresh();
        assert!(unify(&mut ctx, err, b).is_ok());
    }

    #[test]
    fn unify_never_with_concrete() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let n = ctx.never();
        let b = ctx.named(dummy(), vec![]);
        assert!(unify(&mut ctx, n, b).is_ok());
    }

    #[test]
    fn unify_never_with_unresolved_does_not_bind() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let n = ctx.never();
        let b = ctx.fresh();
        assert!(unify(&mut ctx, n, b).is_ok());
        // b should still be unresolved
        assert!(!ctx.is_concrete(b));
    }

    #[test]
    fn unify_tuples_same_arity() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a1 = ctx.fresh();
        let a2 = ctx.fresh();
        let b1 = ctx.fresh();
        let b2 = ctx.fresh();
        let ta = ctx.tuple(vec![a1, a2]);
        let tb = ctx.tuple(vec![b1, b2]);
        assert!(unify(&mut ctx, ta, tb).is_ok());
        assert_eq!(ctx.resolve(a1), ctx.resolve(b1));
        assert_eq!(ctx.resolve(a2), ctx.resolve(b2));
    }

    #[test]
    fn unify_tuples_different_arity() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let e1 = ctx.fresh();
        let ta = ctx.tuple(vec![e1]);
        let e2 = ctx.fresh();
        let e3 = ctx.fresh();
        let tb = ctx.tuple(vec![e2, e3]);
        assert!(matches!(unify(&mut ctx, ta, tb), Err(UnifyError::Mismatch)));
    }

    #[test]
    fn literal_guard_blocks_non_conforming() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let lit = ctx.fresh_literal(LiteralKind::Integer);
        let concrete = ctx.named(dummy(), vec![]);
        // NullResolver says nothing conforms -> LiteralGuard
        assert!(matches!(
            unify(&mut ctx, lit, concrete),
            Err(UnifyError::LiteralGuard)
        ));
    }

    #[test]
    fn merge_literal_markers() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);
        let a = ctx.fresh_literal(LiteralKind::Integer);
        let b = ctx.fresh();
        assert!(unify(&mut ctx, a, b).is_ok());
        // b should now have the Integer literal marker
        let resolved = ctx.resolve(a);
        match &ctx.types[resolved.0 as usize] {
            TySlot::Unresolved {
                literal: Some(LiteralKind::Integer),
            } => {},
            other => panic!("expected Integer literal, got {:?}", other),
        }
    }

    #[test]
    fn loop_break_resolves_to_unit() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);

        // Simulate: loop { break } — break unifies () with the loop's type var
        let break_tv = ctx.fresh();
        ctx.loop_break_tys.push((None, break_tv));
        let unit_tv = ctx.tuple(vec![]);
        assert!(unify(&mut ctx, unit_tv, break_tv).is_ok());
        ctx.loop_break_tys.pop();

        assert!(ctx.is_concrete(break_tv));
        assert!(
            matches!(ctx.slot(break_tv), TySlot::Resolved(TyKind::Tuple(elems)) if elems.is_empty())
        );
    }

    #[test]
    fn loop_no_break_stays_unresolved() {
        let resolver = NullResolver;
        let world = make_world();
        let qctx = world.query_context();
        let mut ctx = make_ctx(&resolver, &qctx);

        // Simulate: loop {} (no break) — type var stays unresolved
        let break_tv = ctx.fresh();
        ctx.loop_break_tys.push((None, break_tv));
        ctx.loop_break_tys.pop();

        assert!(!ctx.is_concrete(break_tv));
    }
}
