//! Bound-aware conformance checking.
//!
//! [`type_satisfies`] answers "does this concrete type *genuinely* satisfy this
//! protocol, including the conformance's `where` clauses?" — unlike the
//! unconditional [`crate::resolve::TypeResolver::conforms_to`] /
//! `ConformingProtocols`, which only check that a conformance is *declared*.
//!
//! It is the single source of truth for conditional-conformance evaluation.
//! Both the analyzer (E616 / `@main` return checking, conformance
//! completeness) and the solver (`solve_conforms`, extension selection) call
//! it, so a `Result[NotExitable, E]` used where `Exitable` is required becomes
//! a clean diagnostic instead of a mono ICE on the missing `report()` witness.
//!
//! `Copyable` / `Cloneable` are answered here too, by delegating to the
//! copy-semantics classifier (`hir_type_copy_semantics`) rather than to
//! `ConformingProtocols` — see the arm in [`type_satisfies_at_depth`]. That
//! keeps this query's answer identical to `TypeResolver::conforms_to`'s and
//! folds a `where T: Copyable` bound into the ordinary bound-evaluation path
//! instead of leaving it unenforced. The solver's `type_conforms_copyable`
//! remains the *per-instantiation* authority; this is the completeness gate,
//! and it permits every abstract position.
//!
//! Both are thin wrappers over the `kestrel-entail` kernel (bidi design §7)
//! through [`crate::entail_layer::FrontEndLayer`]; the evaluation logic lives
//! there, shared with the solver.
//!
//! ## Conservative by design
//!
//! [`type_satisfies`] rejects **only** on a *provable* violation (`Tri::Fails`).
//! The kernel's `Unknown` (an `Infer` slot, an abstract param whose bound is
//! not declared) is permitted here — the analyzer's types are complete, so
//! Unknown can only come from positions this vantage cannot judge. This is
//! load-bearing:
//!
//! * For E616 the `@main` return type is concrete, so a real violation is
//!   caught (`Result[NotExitable, E]`), while resolution errors defer.
//! * For the solver it runs *after* the unconditional `conforms_to` already
//!   confirmed the conformance is declared — so it can only ever turn a
//!   would-be mono ICE into a clean `false`, never spuriously reject a generic
//!   body whose bound is satisfied abstractly.

use kestrel_entail::{Goal, Tri};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::ty::HirTy;

use crate::entail_layer::FrontEndLayer;

/// Does `ty` genuinely satisfy `protocol`, evaluating any conditional
/// conformance `where` clauses (equality clauses included)?
///
/// Protocol identity is by `Entity` (protocol type-args ignored), matching
/// `TypeResolver::conforms_to`. See the module docs for the conservative
/// rejection rule.
pub fn type_satisfies(ctx: &QueryContext<'_>, ty: &HirTy, protocol: Entity, root: Entity) -> bool {
    let layer = FrontEndLayer::new(ctx, root);
    let goal = Goal::Conforms {
        ty: layer.hir_to_ety(ty),
        protocol,
    };
    kestrel_entail::entails(&layer, &goal) != Tri::Fails
}

/// [`type_satisfies`] for a resolved type. Concrete nominals and tuples are
/// reproduced faithfully (so a real violation can be disproven); abstract
/// positions collapse to `Infer`, which `type_satisfies` permits — its
/// conservative "reject only on a provable concrete violation" contract.
pub fn resolved_satisfies(
    ctx: &QueryContext<'_>,
    ty: &crate::result::ResolvedTy,
    protocol: Entity,
    root: Entity,
) -> bool {
    type_satisfies(ctx, &resolved_ty_to_hir(ty), protocol, root)
}

/// A `ResolvedTy` as a `HirTy` for [`type_satisfies`]. Spans are irrelevant
/// to the check. `Struct` vs `Enum` doesn't matter: `nominal_satisfies` keys
/// off the entity's own `NodeKind`.
fn resolved_ty_to_hir(ty: &crate::result::ResolvedTy) -> HirTy {
    use crate::result::ResolvedTy;
    let sp = kestrel_span::Span::new(0, 0..0);
    match ty {
        ResolvedTy::Named { entity, args } => HirTy::Struct {
            entity: *entity,
            args: args.iter().map(resolved_ty_to_hir).collect(),
            span: sp,
        },
        ResolvedTy::Tuple(elems) => {
            HirTy::Tuple(elems.iter().map(resolved_ty_to_hir).collect(), sp)
        },
        _ => HirTy::Infer(sp),
    }
}

/// The first of an existential's `bounds` that `target` cannot conform to:
/// `x is T` / `x as? T` can then never succeed (E149, and MIR folds the test
/// to `false`). `None` when every bound may hold.
pub fn type_test_blocker(
    ctx: &QueryContext<'_>,
    target: &crate::result::ResolvedTy,
    bounds: &[Entity],
    root: Entity,
) -> Option<Entity> {
    bounds
        .iter()
        .copied()
        .find(|&p| !resolved_satisfies(ctx, target, p, root))
}

/// Do `extension`'s target pattern and `where` clauses hold for the receiver
/// type `recv`? The substitution maps the *extension's own* target args (e.g.
/// `[T, E]` in `extend Result[T,E]`) against `recv`'s args — not the target
/// type's declared params, which only coincide positionally for the simple
/// generic case.
pub fn extension_bounds_hold(
    ctx: &QueryContext<'_>,
    extension: Entity,
    recv: &HirTy,
    root: Entity,
) -> bool {
    let layer = FrontEndLayer::new(ctx, root);
    let recv = layer.hir_to_ety(recv);
    let target = layer.extension_target(extension);
    let clauses = layer.extension_clauses(extension);
    kestrel_entail::extension_applies(&layer, &target, &clauses, &recv) != Tri::Fails
}
