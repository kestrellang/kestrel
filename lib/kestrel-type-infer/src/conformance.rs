//! Bound-aware conformance checking.
//!
//! [`type_satisfies`] answers "does this concrete type *genuinely* satisfy this
//! protocol, including the conformance's `where` clauses?" — unlike the
//! unconditional [`crate::resolve::TypeResolver::conforms_to`] /
//! `ConformingProtocols`, which only check that a conformance is *declared*.
//!
//! It is the single source of truth for conditional-conformance evaluation
//! outside the hardcoded Copyable/Cloneable path (`type_conforms_copyable` in
//! the solver). Both the analyzer (E616 / `@main` return checking) and the
//! solver (`solve_conforms`) call it, so a `Result[NotExitable, E]` used where
//! `Exitable` is required becomes a clean diagnostic instead of a mono ICE on
//! the missing `report()` witness.
//!
//! ## Conservative by design
//!
//! [`type_satisfies`] rejects **only** on a *provable concrete* violation.
//! Abstract / generic / unknown positions are permitted. This is load-bearing:
//!
//! * For E616 the `@main` return type is concrete, so a real violation is
//!   caught (`Result[NotExitable, E]`), while resolution errors defer.
//! * For the solver it runs *after* the unconditional `conforms_to` already
//!   confirmed the conformance is declared — so it can only ever turn a
//!   would-be mono ICE into a clean `false`, never spuriously reject a generic
//!   body whose bound is satisfied abstractly.

use kestrel_ast_builder::NodeKind;
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::builtin::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerExtensionTargetTypeArgs;
use kestrel_name_res::{
    ConformingProtocolInstantiations, ConformingProtocols, ExtensionTargetEntity, ResolveBuiltin,
};

use crate::resolve::WhereClause;
use crate::where_clauses::WhereClausesOf;

/// Does `ty` genuinely satisfy `protocol`, evaluating any conditional
/// conformance `where` clauses?
///
/// Protocol identity is by `Entity` (protocol type-args ignored), matching
/// `TypeResolver::conforms_to`. See the module docs for the conservative
/// rejection rule.
pub fn type_satisfies(ctx: &QueryContext<'_>, ty: &HirTy, protocol: Entity, root: Entity) -> bool {
    match ty {
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            nominal_satisfies(ctx, ty, *entity, args, protocol, root)
        },
        // Structural singletons conform via their synthetic `lang` entities
        // (`extend (): P` / `extend !: P`), keyed the same as nominal types.
        HirTy::Tuple(elems, _) if elems.is_empty() => {
            match kestrel_name_res::extensions::resolve_lang_child(ctx, root, "()") {
                Some(e) => nominal_satisfies(ctx, ty, e, &[], protocol, root),
                None => true,
            }
        },
        HirTy::Never(_) => match kestrel_name_res::extensions::resolve_lang_child(ctx, root, "!") {
            Some(e) => nominal_satisfies(ctx, ty, e, &[], protocol, root),
            None => true,
        },
        // Param / SelfType / AssocProjection / Opaque / Function / AliasUse /
        // non-empty Tuple / Infer / Error: nothing concrete to disprove here, so
        // permit. Abstract positions MUST be permitted so generic bodies aren't
        // spuriously rejected (the conservative rule).
        _ => true,
    }
}

/// Do `extension`'s `where` clauses hold for the receiver type `recv`?
///
/// The single robust bound-evaluator. The substitution maps the *extension's
/// own* target args (e.g. `[T, E]` in `extend Result[T,E]`) against `recv`'s
/// args — not the target type's declared params, which only coincide
/// positionally for the simple generic case and not at all for specialized /
/// free-param extensions.
pub fn extension_bounds_hold(
    ctx: &QueryContext<'_>,
    extension: Entity,
    recv: &HirTy,
    root: Entity,
) -> bool {
    let target_args = ctx
        .query(LowerExtensionTargetTypeArgs { extension, root })
        .unwrap_or_default();
    extension_bounds_hold_impl(ctx, extension, &target_args, recv, hir_args(recv), root)
}

/// Nominal `entity[args]` conformance to `protocol`, bounds included.
fn nominal_satisfies(
    ctx: &QueryContext<'_>,
    recv: &HirTy,
    entity: Entity,
    args: &[HirTy],
    protocol: Entity,
    root: Entity,
) -> bool {
    // Must at least *declare* the conformance (closure-aware: inheritance,
    // extension-added, refinement). A plain non-conformer fails here.
    if !ctx
        .query(ConformingProtocols { entity, root })
        .contains(&protocol)
    {
        return false;
    }

    // Find the most-specific extension that *supplies* this conformance with a
    // `where` clause. A direct (non-extension) conformance is unconditional.
    let insts = ctx.query(ConformingProtocolInstantiations { entity, root });
    let mut best: Option<(Entity, Vec<HirTy>, usize)> = None; // (ext, target_args, specificity)
    for (proto, source, _proto_args) in &insts {
        if *proto != protocol {
            continue;
        }
        if ctx.get::<NodeKind>(*source) != Some(&NodeKind::Extension) {
            // Direct / inherited conformance — unconditional from our vantage.
            return true;
        }
        let target_args = ctx
            .query(LowerExtensionTargetTypeArgs {
                extension: *source,
                root,
            })
            .unwrap_or_default();
        // A specialized extension applies only if its concrete target positions
        // structurally match the instance args.
        if !target_args_apply(&target_args, args) {
            continue;
        }
        let specificity = target_args.iter().filter(|t| !is_param(t)).count();
        if best.as_ref().is_none_or(|(_, _, s)| specificity > *s) {
            best = Some((*source, target_args, specificity));
        }
    }

    let Some((source, target_args, _)) = best else {
        // Declares is true but no direct extension source matched (e.g. the
        // conformance arrived via protocol closure/refinement). Defer to the
        // declares result; never reject on incompleteness.
        return true;
    };

    extension_bounds_hold_impl(ctx, source, &target_args, recv, args, root)
}

fn extension_bounds_hold_impl(
    ctx: &QueryContext<'_>,
    extension: Entity,
    target_args: &[HirTy],
    recv: &HirTy,
    recv_args: &[HirTy],
    root: Entity,
) -> bool {
    let clauses = ctx.query(WhereClausesOf { entity: extension, root });
    if clauses.is_empty() {
        return true;
    }

    // Extension param entity → concrete instance arg, from the Param positions
    // of the extension's own target args.
    let subst: Vec<(Entity, &HirTy)> = target_args
        .iter()
        .zip(recv_args.iter())
        .filter_map(|(t, c)| match t {
            HirTy::Param(e, _) => Some((*e, c)),
            _ => None,
        })
        .collect();

    // `where Self: Q` — the clause subject is the extension's target entity.
    let target_entity = ctx.query(ExtensionTargetEntity { extension, root });

    for clause in &clauses {
        let WhereClause::Bound {
            param,
            protocol: pb,
            ..
        } = clause
        else {
            continue; // TypeEquality / DirectEquality — out of scope, treat satisfied.
        };
        // Copyable / Cloneable are copy-semantics, not declared conformances;
        // `type_satisfies` (which goes through `ConformingProtocols`) can't
        // answer them. Skip — copyability is enforced by the move checker / mono.
        if is_copy_builtin(ctx, *pb, root) {
            continue;
        }
        let sub_ty = if let Some((_, c)) = subst.iter().find(|(e, _)| e == param) {
            (*c).clone()
        } else if Some(*param) == target_entity {
            recv.clone()
        } else {
            continue; // Unknown param — permit (conservative).
        };
        if !type_satisfies(ctx, &sub_ty, *pb, root) {
            return false;
        }
    }
    true
}

/// Every concrete (non-`Param`) position of `target_args` structurally matches
/// the corresponding instance arg. Generic extensions (all-`Param`) always apply.
fn target_args_apply(target_args: &[HirTy], args: &[HirTy]) -> bool {
    target_args
        .iter()
        .zip(args.iter())
        .all(|(t, a)| is_param(t) || hir_ty_matches(t, a))
}

/// Structural equality of two `HirTy`s, ignoring spans. A `Param` pattern
/// matches anything (it's a placeholder, not a concrete requirement).
fn hir_ty_matches(pattern: &HirTy, concrete: &HirTy) -> bool {
    match (pattern, concrete) {
        (HirTy::Param(..), _) => true,
        (
            HirTy::Struct {
                entity: pe,
                args: pa,
                ..
            },
            HirTy::Struct {
                entity: ce,
                args: ca,
                ..
            },
        )
        | (
            HirTy::Enum {
                entity: pe,
                args: pa,
                ..
            },
            HirTy::Enum {
                entity: ce,
                args: ca,
                ..
            },
        )
        | (
            HirTy::Protocol {
                entity: pe,
                args: pa,
                ..
            },
            HirTy::Protocol {
                entity: ce,
                args: ca,
                ..
            },
        ) => pe == ce && pa.len() == ca.len() && pa.iter().zip(ca).all(|(x, y)| hir_ty_matches(x, y)),
        (HirTy::Tuple(pe, _), HirTy::Tuple(ce, _)) => {
            pe.len() == ce.len() && pe.iter().zip(ce).all(|(x, y)| hir_ty_matches(x, y))
        },
        (HirTy::Never(_), HirTy::Never(_)) => true,
        _ => false,
    }
}

fn is_param(ty: &HirTy) -> bool {
    matches!(ty, HirTy::Param(..))
}

/// The nominal/structural type args of `ty` (empty for non-parameterized types).
fn hir_args(ty: &HirTy) -> &[HirTy] {
    match ty {
        HirTy::Struct { args, .. }
        | HirTy::Enum { args, .. }
        | HirTy::Protocol { args, .. }
        | HirTy::AliasUse { args, .. } => args,
        HirTy::Tuple(elems, _) => elems,
        _ => &[],
    }
}

fn is_copy_builtin(ctx: &QueryContext<'_>, protocol: Entity, root: Entity) -> bool {
    [Builtin::Copyable, Builtin::Cloneable]
        .into_iter()
        .any(|builtin| ctx.query(ResolveBuiltin { builtin, root }) == Some(protocol))
}
