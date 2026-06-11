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
    type_satisfies_at_depth(ctx, ty, protocol, root, 0)
}

/// Depth-threaded worker. Two recursion axes meet here: structural (bound
/// evaluation walks into type args) and refinement (a protocol-sourced
/// conformance recurses on the PARENT protocol). Protocol cycles are
/// rejected elsewhere (E459), but this query can run before that
/// diagnostic fires — the guard falls back to the conservative permit.
fn type_satisfies_at_depth(
    ctx: &QueryContext<'_>,
    ty: &HirTy,
    protocol: Entity,
    root: Entity,
    depth: u32,
) -> bool {
    if depth > 32 {
        return true;
    }
    // Static is structural, never declared — the ConformingProtocols walk
    // below can't answer it. The HIR staticness walk shares this function's
    // conservative contract exactly (abstract positions permit).
    if ctx.query(ResolveBuiltin {
        builtin: Builtin::Static,
        root,
    }) == Some(protocol)
    {
        // No asking-site context exists here; `root` is fine — the
        // requirement query always consults the param's own declaring
        // parent first (same scoping the solver's Param arm uses).
        return kestrel_semantics::hir_type_is_static(ctx, ty, root, root);
    }
    match ty {
        // A REF is concrete: it satisfies Copyable (bit-copy, stage 2b
        // ruling) and NOTHING else — permitting it here would let an
        // extension bound (`extend Optional: Equatable where T: Equatable`)
        // instantiate a pointee witness at `&U` and ICE at mono
        // ("Callee::Witness not resolved", the witness_instantiation_
        // collapse class). The Expr-side transparent place never reaches
        // this check (solve_conforms peels first). Real ref witnesses
        // are 2d.
        HirTy::Ref { .. } => {
            kestrel_debug::ktrace!("ref-gate", "type_satisfies(Ref, {protocol:?})");
            ctx.query(ResolveBuiltin {
                builtin: Builtin::Copyable,
                root,
            }) == Some(protocol)
        },
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            nominal_satisfies(ctx, ty, *entity, args, protocol, root, depth)
        },
        // Structural singletons conform via their synthetic `lang` entities
        // (`extend (): P` / `extend !: P`), keyed the same as nominal types.
        HirTy::Tuple(elems, _) if elems.is_empty() => {
            match kestrel_name_res::extensions::resolve_lang_child(ctx, root, "()") {
                Some(e) => nominal_satisfies(ctx, ty, e, &[], protocol, root, depth),
                None => true,
            }
        },
        HirTy::Never(_) => match kestrel_name_res::extensions::resolve_lang_child(ctx, root, "!") {
            Some(e) => nominal_satisfies(ctx, ty, e, &[], protocol, root, depth),
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
    extension_bounds_hold_impl(ctx, extension, &target_args, recv, hir_args(recv), root, 0)
}

/// Nominal `entity[args]` conformance to `protocol`, bounds included.
fn nominal_satisfies(
    ctx: &QueryContext<'_>,
    recv: &HirTy,
    entity: Entity,
    args: &[HirTy],
    protocol: Entity,
    root: Entity,
    depth: u32,
) -> bool {
    // Must at least *declare* the conformance (closure-aware: inheritance,
    // extension-added, refinement). A plain non-conformer fails here.
    if !ctx
        .query(ConformingProtocols { entity, root })
        .contains(&protocol)
    {
        return false;
    }

    // A protocol-typed receiver is abstract — nothing concrete to disprove
    // (same conservative rule as Param/Opaque in `type_satisfies`).
    if ctx.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
        return true;
    }

    // Classify the sources that *supply* this conformance. A conformance in
    // the type's own decl header is unconditional. An Extension on the TYPE
    // carries evaluable `where` clauses (most-specific applicable wins). Two
    // source shapes arrive INDIRECTLY through another protocol and hold only
    // if the receiver genuinely satisfies that parent — recurse instead of
    // trusting the declaration:
    //   * a PROTOCOL source (refinement: `protocol Comparable: Equatable`
    //     attributes Equatable to the Comparable entity), and
    //   * an extension whose TARGET is a protocol (blanket:
    //     `extend Equatable: Equal[Self]` supplies Equal to everything
    //     Equatable — gate on Equatable, not on the blanket's empty clauses).
    // Trusting them was the stage-2b operator gap: `Optional[&Int64] == …`
    // accepted Equal via the blanket, never evaluated `extend Optional[T]:
    // Equatable where T: Equatable`, and ICEd post-mono on the missing
    // `&Int64` witness. (The blanket's own where clauses are NOT evaluated
    // here: its params don't positionally map onto the receiver's args, and
    // before this fix they were never consulted either — the parent-protocol
    // gate is the genuine requirement.)
    let insts = ctx.query(ConformingProtocolInstantiations { entity, root });
    let mut best: Option<(Entity, Vec<HirTy>, usize)> = None; // (ext, target_args, specificity)
    let mut parent_protocols: Vec<Entity> = Vec::new();
    for (proto, source, _proto_args) in &insts {
        if *proto != protocol {
            continue;
        }
        match ctx.get::<NodeKind>(*source) {
            Some(NodeKind::Extension) => {
                if let Some(target) = ctx.query(ExtensionTargetEntity {
                    extension: *source,
                    root,
                }) && ctx.get::<NodeKind>(target) == Some(&NodeKind::Protocol)
                {
                    parent_protocols.push(target);
                    continue;
                }
                let target_args = ctx
                    .query(LowerExtensionTargetTypeArgs {
                        extension: *source,
                        root,
                    })
                    .unwrap_or_default();
                // A specialized extension applies only if its concrete target
                // positions structurally match the instance args.
                if !target_args_apply(&target_args, args) {
                    continue;
                }
                let specificity = target_args.iter().filter(|t| !is_param(t)).count();
                if best.as_ref().is_none_or(|(_, _, s)| specificity > *s) {
                    best = Some((*source, target_args, specificity));
                }
            },
            Some(NodeKind::Protocol) => parent_protocols.push(*source),
            // The type decl itself (or another non-evaluable source) —
            // unconditional from our vantage.
            _ => return true,
        }
    }

    if let Some((source, target_args, _)) = &best
        && extension_bounds_hold_impl(ctx, *source, target_args, recv, args, root, depth)
    {
        return true;
    }
    // Indirect supply: holds iff the receiver genuinely satisfies a parent
    // protocol that carries it.
    if parent_protocols
        .iter()
        .any(|&parent| type_satisfies_at_depth(ctx, recv, parent, root, depth + 1))
    {
        return true;
    }
    // No evaluable source said yes. Reject only when something evaluable
    // said no (a provable violation); with no applicable extension and no
    // parent protocol, defer to the declares result — never reject on
    // incompleteness.
    best.is_none() && parent_protocols.is_empty()
}

fn extension_bounds_hold_impl(
    ctx: &QueryContext<'_>,
    extension: Entity,
    target_args: &[HirTy],
    recv: &HirTy,
    recv_args: &[HirTy],
    root: Entity,
    depth: u32,
) -> bool {
    let clauses = ctx.query(WhereClausesOf {
        entity: extension,
        root,
    });
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
        if !type_satisfies_at_depth(ctx, &sub_ty, *pb, root, depth + 1) {
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
        ) => {
            pe == ce && pa.len() == ca.len() && pa.iter().zip(ca).all(|(x, y)| hir_ty_matches(x, y))
        },
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
