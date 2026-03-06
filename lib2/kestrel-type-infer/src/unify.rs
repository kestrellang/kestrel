//! Unification with literal guards, Error/Never handling.
//!
//! The core operation: make two TyVars equal by binding one to the other
//! or recursively unifying their structure. Special handling for:
//! - Error types: absorb silently (prevent cascading errors)
//! - Never types: unify with anything (bottom type)
//! - Literal markers: guard against unification with non-conforming types

use crate::ctx::InferCtx;
use kestrel_hir::Builtin;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};

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
        // If the other side is Unresolved, don't bind — let other constraints resolve it.
        (TySlot::Resolved(TyKind::Never), TySlot::Unresolved { .. })
        | (TySlot::Unresolved { .. }, TySlot::Resolved(TyKind::Never)) => Ok(()),
        (TySlot::Resolved(TyKind::Never), _) | (_, TySlot::Resolved(TyKind::Never)) => Ok(()),

        // Both unresolved: link them, merge literal markers.
        (
            TySlot::Unresolved { literal: lit_a },
            TySlot::Unresolved { literal: lit_b },
        ) => {
            let merged = lit_a.or(*lit_b);
            ctx.types[a.0 as usize] = TySlot::Redirect(b);
            if merged.is_some() {
                ctx.types[b.0 as usize] = TySlot::Unresolved { literal: merged };
            }
            Ok(())
        }

        // Unresolved (non-literal) + Concrete: bind
        (TySlot::Unresolved { literal: None }, _) => {
            occurs_check(ctx, a, b)?;
            ctx.types[a.0 as usize] = TySlot::Redirect(b);
            Ok(())
        }
        (_, TySlot::Unresolved { literal: None }) => {
            occurs_check(ctx, b, a)?;
            ctx.types[b.0 as usize] = TySlot::Redirect(a);
            Ok(())
        }

        // Literal TyVar + Concrete: guard with ExpressibleBy* conformance
        (TySlot::Unresolved { literal: Some(lit) }, TySlot::Resolved(kind)) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, a, b)?;
                ctx.types[a.0 as usize] = TySlot::Redirect(b);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        }
        (TySlot::Resolved(kind), TySlot::Unresolved { literal: Some(lit) }) => {
            if conforms_to_literal_protocol(ctx, kind, *lit) {
                occurs_check(ctx, b, a)?;
                ctx.types[b.0 as usize] = TySlot::Redirect(a);
                Ok(())
            } else {
                Err(UnifyError::LiteralGuard)
            }
        }

        // Both concrete: structural unification
        (TySlot::Resolved(kind_a), TySlot::Resolved(kind_b)) => {
            unify_concrete(ctx, kind_a, kind_b)
        }

        // Redirect should be resolved by resolve()
        _ => unreachable!("resolve() should have followed redirects"),
    }
}

/// Structural unification of two concrete types.
fn unify_concrete(
    ctx: &mut InferCtx<'_>,
    a: &TyKind,
    b: &TyKind,
) -> Result<(), UnifyError> {
    match (a, b) {
        // Named types: same entity + unify type args pairwise
        (
            TyKind::Named { entity: ea, args: aa },
            TyKind::Named { entity: eb, args: ab },
        ) => {
            if ea != eb || aa.len() != ab.len() {
                return Err(UnifyError::Mismatch);
            }
            // Clone args to avoid borrow issues
            let pairs: Vec<(TyVar, TyVar)> =
                aa.iter().copied().zip(ab.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            Ok(())
        }

        // Tuples: same arity + unify elements pairwise
        (TyKind::Tuple(ea), TyKind::Tuple(eb)) => {
            if ea.len() != eb.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> =
                ea.iter().copied().zip(eb.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            Ok(())
        }

        // Functions: same arity + unify params + unify return
        (
            TyKind::Function { params: pa, ret: ra },
            TyKind::Function { params: pb, ret: rb },
        ) => {
            if pa.len() != pb.len() {
                return Err(UnifyError::Mismatch);
            }
            let pairs: Vec<(TyVar, TyVar)> =
                pa.iter().copied().zip(pb.iter().copied()).collect();
            for (a, b) in pairs {
                unify(ctx, a, b)?;
            }
            unify(ctx, *ra, *rb)
        }

        // Type params: must be the same entity
        (TyKind::Param { entity: a }, TyKind::Param { entity: b }) => {
            if a == b {
                Ok(())
            } else {
                Err(UnifyError::Mismatch)
            }
        }

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
        TySlot::Resolved(TyKind::Named { args, .. }) => {
            for &arg in args {
                occurs_check(ctx, tv, arg)?;
            }
            Ok(())
        }
        TySlot::Resolved(TyKind::Tuple(elems)) => {
            for &e in elems {
                occurs_check(ctx, tv, e)?;
            }
            Ok(())
        }
        TySlot::Resolved(TyKind::Function { params, ret }) => {
            for &p in params {
                occurs_check(ctx, tv, p)?;
            }
            occurs_check(ctx, tv, *ret)
        }
        _ => Ok(()),
    }
}

/// Check if a concrete type conforms to the literal's ExpressibleBy* protocol.
pub fn conforms_to_literal_protocol(
    ctx: &InferCtx<'_>,
    ty: &TyKind,
    lit: LiteralKind,
) -> bool {
    let feature = match lit {
        LiteralKind::Integer => Builtin::ExpressibleByIntegerLiteral,
        LiteralKind::Float => Builtin::ExpressibleByFloatLiteral,
        LiteralKind::String => Builtin::ExpressibleByStringLiteral,
        LiteralKind::Bool => Builtin::ExpressibleByBoolLiteral,
        LiteralKind::Char => Builtin::ExpressibleByCharLiteral,
        LiteralKind::Null => Builtin::ExpressibleByNullLiteral,
        LiteralKind::Array => Builtin::ExpressibleByArrayLiteral,
        LiteralKind::Dictionary => Builtin::ExpressibleByDictionaryLiteral,
    };
    let Some(protocol) = ctx.resolver.builtin(feature) else {
        return false;
    };
    ctx.resolver.conforms_to(ty, protocol)
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
        fn conforms_to(&self, _: &TyKind, _: Entity) -> bool {
            false
        }
        fn resolve_associated_type(&self, _: &TyKind, _: &str) -> Option<AssociatedTypeResolution> {
            None
        }
        fn builtin(&self, _: Builtin) -> Option<Entity> {
            None
        }
        fn where_clauses(&self, _: Entity) -> Vec<WhereClause> {
            Vec::new()
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

    fn make_ctx<'a>(resolver: &'a dyn TypeResolver, qctx: &'a kestrel_hecs::QueryContext<'a>) -> InferCtx<'a> {
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
            span: kestrel_span2::Span::synthetic(0),
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
            TySlot::Unresolved { literal: Some(LiteralKind::Integer) } => {}
            other => panic!("expected Integer literal, got {:?}", other),
        }
    }
}
