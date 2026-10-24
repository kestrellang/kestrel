//! Existential conversions at check leaves (existentials design §4).
//!
//! With an expected type `B[rawany P]`, a value of another type converts by
//! one of three rules, and anything else is an error:
//!
//! | found               | rule    |
//! |---------------------|---------|
//! | sized `T`, `T: P`   | Box     |
//! | `B[T]`, `T: P`      | Unsize  |
//! | `B[rawany Q]`, `Q ⇒ P` | Upcast |
//! | `C[rawany Q]`, `C ≠ B` | E147 |
//!
//! The rule is recorded on the expression (`InferCtx::existential_conversions`)
//! for MIR lowering, the way `FromValue` promotions are.

use kestrel_hecs::Entity;
use kestrel_semantics::ProtocolRefines;
use kestrel_span::Span;

use crate::constraint::AssocKey;
use crate::ctx::InferCtx;
use crate::error::InferError;
use crate::result::{ConversionKind, describe_tyvar};
use crate::solver::{SolveResult, reify_tv, solve_associated};
use crate::ty::{TyKind, TySlot, TyVar};
use crate::unify;

/// A conversion accepted during solving; resolved into
/// `result::ExistentialConversion` when the body is done.
#[derive(Clone, Debug)]
pub(crate) struct PendingConversion {
    pub kind: ConversionKind,
    pub from: TyVar,
    pub to: TyVar,
}

/// The parts of an existential type `B[rawany P and Q, A = X]`.
struct Existential {
    box_entity: Entity,
    bounds: Vec<Entity>,
    bindings: Vec<(Entity, TyVar)>,
}

/// `tv` as an existential, if it resolves to `B[rawany …]`.
fn as_existential(ctx: &InferCtx<'_>, tv: TyVar) -> Option<Existential> {
    let TySlot::Resolved(TyKind::Struct { entity, args } | TyKind::Enum { entity, args }) =
        ctx.slot(ctx.resolve(tv))
    else {
        return None;
    };
    let [arg] = args.as_slice() else {
        return None;
    };
    let TySlot::Resolved(TyKind::RawExistential { bounds, bindings }) = ctx.slot(ctx.resolve(*arg))
    else {
        return None;
    };
    Some(Existential {
        box_entity: *entity,
        bounds: bounds.iter().map(|(p, _)| *p).collect(),
        bindings: bindings.clone(),
    })
}

/// Try to convert `from` to the existential `to`. `None` when `to` is not an
/// existential, or no rule applies and the caller's ordinary mismatch should
/// be reported. Both sides must already be concrete.
pub(crate) fn try_convert(
    ctx: &mut InferCtx<'_>,
    from: TyVar,
    to: TyVar,
    expr: kestrel_hir::body::HirExprId,
    span: &Span,
) -> Option<SolveResult> {
    let target = as_existential(ctx, to)?;
    let kind = match as_existential(ctx, from) {
        Some(source) => {
            if source.box_entity != target.box_entity {
                return Some(SolveResult::Error(InferError::ExistentialBoxMismatch {
                    from: describe_tyvar(ctx, from),
                    to: describe_tyvar(ctx, to),
                    span: span.clone(),
                }));
            }
            if !upcast_holds(ctx, &source, &target) {
                return None;
            }
            ConversionKind::Upcast
        },
        None => {
            // `B[T]` is unsized in place; anything else is boxed whole.
            let (kind, payload) = match ctx.slot(ctx.resolve(from)) {
                TySlot::Resolved(TyKind::Struct { entity, args } | TyKind::Enum { entity, args })
                    if *entity == target.box_entity && args.len() == 1 =>
                {
                    (ConversionKind::Unsize, args[0])
                },
                _ => (ConversionKind::Box, from),
            };
            if let Some(err) = payload_error(ctx, payload, from, to, &target, span) {
                return Some(err);
            }
            kind
        },
    };
    ctx.existential_conversions
        .insert(expr, PendingConversion { kind, from, to });
    Some(SolveResult::Solved)
}

/// Does `source` (the same box) imply every bound and binding of `target`?
/// A bound is implied when some source bound refines it (a parent, or a
/// member of a composition); a binding must be pinned to the same type.
fn upcast_holds(ctx: &mut InferCtx<'_>, source: &Existential, target: &Existential) -> bool {
    let implied = target.bounds.iter().all(|&base| {
        source.bounds.iter().any(|&protocol| {
            ctx.query_ctx.query(ProtocolRefines {
                protocol,
                base,
                root: ctx.root,
            })
        })
    });
    implied
        && target.bindings.iter().all(|(assoc, want)| {
            source
                .bindings
                .iter()
                .find(|(a, _)| a == assoc)
                .is_some_and(|(_, have)| unify::unify(ctx, *have, *want).is_ok())
        })
}

/// Why `payload` can't be the payload of `target`, if it can't: it must
/// conform to every bound and have every pinned associated type.
fn payload_error(
    ctx: &mut InferCtx<'_>,
    payload: TyVar,
    from: TyVar,
    to: TyVar,
    target: &Existential,
    span: &Span,
) -> Option<SolveResult> {
    let payload_hir = reify_tv(ctx, payload);
    for &protocol in &target.bounds {
        if !crate::conformance::type_satisfies(ctx.query_ctx, &payload_hir, protocol, ctx.root) {
            return Some(SolveResult::Error(InferError::DoesNotConform {
                ty: payload,
                protocol,
                span: span.clone(),
            }));
        }
    }
    for &(assoc, want) in &target.bindings {
        let actual = ctx.fresh();
        match solve_associated(ctx, payload, AssocKey::Entity(assoc), actual, span.clone()) {
            SolveResult::Solved => {},
            // Not decidable yet: keep the binding as a plain equality.
            SolveResult::Deferred(c) => {
                ctx.constraints.push(c);
                ctx.equal(actual, want, span.clone());
                continue;
            },
            err @ SolveResult::Error(_) => return Some(err),
        }
        if unify::unify(ctx, actual, want).is_err() {
            return Some(SolveResult::Error(InferError::ExistentialBindingMismatch {
                from: describe_tyvar(ctx, from),
                to: describe_tyvar(ctx, to),
                assoc: crate::resolve::assoc_name(ctx.query_ctx, assoc),
                actual: describe_tyvar(ctx, actual),
                span: span.clone(),
            }));
        }
    }
    None
}

// ===== Member lookup (existentials design §7) =====

/// Protocol-first lookup on an existential receiver `B[rawany P]`. When `P`
/// (or a parent) has a member `name`, check that it can be called through
/// this existential (E145, E148) and peel the box, returning the payload
/// type to look the member up on. `None` when the receiver is not an
/// existential or `P` has no such member: the box's own members are tried
/// next, so its API stays reachable when nothing in `P` shadows it.
pub(crate) fn peel_for_member(
    ctx: &mut InferCtx<'_>,
    receiver: TyVar,
    recv_kind: &TyKind,
    name: &str,
    args: &[crate::constraint::CallArg],
    expr: kestrel_hir::body::HirExprId,
    span: &Span,
) -> Option<Result<TyVar, InferError>> {
    let existential = as_existential(ctx, receiver)?;
    let (TyKind::Struct { args: box_args, .. } | TyKind::Enum { args: box_args, .. }) = recv_kind
    else {
        return None;
    };
    let raw_kind = match ctx.slot(ctx.resolve(box_args[0])) {
        TySlot::Resolved(k) => k.clone(),
        _ => return None,
    };
    let member = ctx
        .resolver
        .resolve_member(&raw_kind, name, args)
        .or_else(|_| ctx.resolver.resolve_static_member(&raw_kind, name, args))
        .ok()?
        .entity;
    // Extension methods take the box as `Self` instead
    // ([`self_conformance_member`]).
    if !kestrel_semantics::is_requirement(ctx.query_ctx, member) {
        return None;
    }
    if let Some(err) = call_error(ctx, receiver, &existential, member, name, span) {
        return Some(Err(err));
    }
    crate::solver::try_indirection_peel(ctx, recv_kind, receiver, expr, span).map(Ok)
}

/// Why `member` can't be called on the existential `receiver`, if it can't.
fn call_error(
    ctx: &InferCtx<'_>,
    receiver: TyVar,
    existential: &Existential,
    member: Entity,
    name: &str,
    span: &Span,
) -> Option<InferError> {
    let q = ctx.query_ctx;
    // Extension methods are reached through self-conformance (§6), not the
    // table, so only requirements are checked here.
    if !kestrel_semantics::is_requirement(q, member) {
        return None;
    }
    let shown = describe_tyvar(ctx, receiver);
    let bound: Vec<Entity> = existential.bindings.iter().map(|(a, _)| *a).collect();
    let dispatch = q.query(kestrel_semantics::RequirementDispatch {
        requirement: member,
        root: ctx.root,
    });
    if !dispatch.allowed(&bound) {
        let reason = match &dispatch {
            kestrel_semantics::Dispatch::Static => "it is a static requirement".to_string(),
            kestrel_semantics::Dispatch::Generic => "it is generic".to_string(),
            kestrel_semantics::Dispatch::UsesSelf => {
                "it uses 'Self' as a parameter or return type".to_string()
            },
            kestrel_semantics::Dispatch::Dispatchable { assocs } => {
                let missing = assocs.iter().find(|a| !bound.contains(a)).copied();
                let assoc = missing.map_or_else(String::new, |a| {
                    crate::resolve::assoc_name(q, a)
                });
                format!("it mentions associated type '{assoc}', which '{shown}' leaves unbound")
            },
        };
        return Some(InferError::NotDispatchable {
            message: format!("'{name}' cannot be called on '{shown}': {reason}"),
            span: span.clone(),
        });
    }
    access_error(ctx, receiver, existential, member, name, &shown, span)
}

/// E148: the box does not grant the access `member`'s receiver needs.
fn access_error(
    ctx: &InferCtx<'_>,
    receiver: TyVar,
    existential: &Existential,
    member: Entity,
    name: &str,
    shown: &str,
    span: &Span,
) -> Option<InferError> {
    use kestrel_ast_builder::{Callable, ReceiverKind};
    use kestrel_hir::Builtin;
    let q = ctx.query_ctx;
    let (mode, needs) = match q.get::<Callable>(member).and_then(|c| c.receiver.clone()) {
        Some(ReceiverKind::Consuming) => ("consuming", Builtin::ConsumingIndirection),
        Some(ReceiverKind::Mutating) => ("mutating", Builtin::MutableIndirection),
        _ => return None,
    };
    let protocol = ctx.resolver.builtin(needs)?;
    let box_hir = reify_tv(ctx, receiver);
    if crate::conformance::type_satisfies(q, &box_hir, protocol, ctx.root) {
        return None;
    }
    let box_name = entity_name(ctx, existential.box_entity);
    // A Cloneable box hands out copies that share the payload.
    let shares = ctx
        .resolver
        .builtin(Builtin::Cloneable)
        .is_some_and(|c| {
            q.query(kestrel_name_res::ConformingProtocols {
                entity: existential.box_entity,
                root: ctx.root,
            })
            .contains(&c)
        });
    let reason = match (mode, shares) {
        ("consuming", true) => format!("'{box_name}' shares its payload, so it cannot give it up"),
        ("consuming", false) => format!("'{box_name}' cannot give up its payload"),
        _ => format!("'{box_name}' gives only read access to its payload"),
    };
    let hint = ctx
        .resolver
        .builtin(Builtin::OwnedBox)
        .filter(|&owned| owned != existential.box_entity)
        .map(|owned| {
            let TySlot::Resolved(TyKind::RawExistential { bounds, bindings }) =
                existential_payload(ctx, receiver)
            else {
                return String::new();
            };
            let bounds = crate::result::describe_existential_bounds(ctx, &bounds, &bindings);
            format!("; use 'any[{}] {bounds}'", entity_name(ctx, owned))
        })
        .unwrap_or_default();
    Some(InferError::ExistentialAccess {
        message: format!("cannot call {mode} method '{name}' on '{shown}': {reason}{hint}"),
        span: span.clone(),
    })
}

/// The payload slot of an existential box.
fn existential_payload(ctx: &InferCtx<'_>, receiver: TyVar) -> TySlot {
    match ctx.slot(ctx.resolve(receiver)) {
        TySlot::Resolved(TyKind::Struct { args, .. } | TyKind::Enum { args, .. }) => {
            ctx.slot(ctx.resolve(args[0])).clone()
        },
        other => other.clone(),
    }
}

fn entity_name(ctx: &InferCtx<'_>, entity: Entity) -> String {
    ctx.query_ctx
        .get::<kestrel_ast_builder::Name>(entity)
        .map_or_else(|| "?".to_string(), |n| n.0.clone())
}

// ===== Self-conformance (existentials design §6) =====

/// Does the existential `tv` name `protocol` among its bounds (or what they
/// refine)? The kernel then decides whether `any P: P` actually holds.
pub(crate) fn names_protocol(ctx: &InferCtx<'_>, tv: TyVar, protocol: Entity) -> bool {
    as_existential(ctx, tv).is_some_and(|e| {
        kestrel_semantics::reachable_protocols(ctx.query_ctx, &e.bounds, ctx.root)
            .contains(&protocol)
    })
}

/// E146 for a failed `existential: protocol` the existential names: say
/// which requirement (or which missing box access) blocks `any P: P`.
pub(crate) fn self_conformance_error(
    ctx: &InferCtx<'_>,
    tv: TyVar,
    protocol: Entity,
    span: &Span,
) -> Option<InferError> {
    use kestrel_hir::Builtin;
    let existential = as_existential(ctx, tv)?;
    if !kestrel_semantics::bound_closure(ctx.query_ctx, &existential.bounds, ctx.root)
        .contains(&protocol)
    {
        return None;
    }
    let pinned: Vec<Entity> = existential.bindings.iter().map(|(a, _)| *a).collect();
    let needs = kestrel_semantics::self_conformance(ctx.query_ctx, protocol, &pinned, ctx.root);
    let shown = describe_tyvar(ctx, tv);
    let proto = entity_name(ctx, protocol);
    let why = match needs.blocker {
        Some(requirement) => format!(
            "requirement '{}' cannot be called through an existential",
            entity_name(ctx, requirement)
        ),
        None => {
            let box_name = entity_name(ctx, existential.box_entity);
            if needs.needs_consuming
                && ctx
                    .resolver
                    .builtin(Builtin::ConsumingIndirection)
                    .is_some_and(|p| {
                        !crate::conformance::type_satisfies(
                            ctx.query_ctx,
                            &reify_tv(ctx, tv),
                            p,
                            ctx.root,
                        )
                    })
            {
                format!("a requirement is 'consuming', and '{box_name}' cannot give up its payload")
            } else {
                format!("a requirement is 'mutating', and '{box_name}' gives only read access")
            }
        },
    };
    Some(InferError::NotSelfConforming {
        message: format!("'{shown}' does not conform to '{proto}': {why}"),
        span: span.clone(),
    })
}

/// A member reached on an existential box through `any P: P` rather than
/// the peel: an extension method of a bound, or any member when the use is
/// an operator / `for-in` (`protocol_dispatch`). Resolved on the payload's
/// protocols; the box stays the receiver, so `Self` is the box.
pub(crate) fn self_conformance_member(
    ctx: &InferCtx<'_>,
    receiver: TyVar,
    recv_kind: &TyKind,
    name: &str,
    args: &[crate::constraint::CallArg],
    protocol_dispatch: bool,
) -> Option<crate::resolve::MemberResolution> {
    as_existential(ctx, receiver)?;
    let (TyKind::Struct { args: box_args, .. } | TyKind::Enum { args: box_args, .. }) = recv_kind
    else {
        return None;
    };
    let TySlot::Resolved(raw_kind) = ctx.slot(ctx.resolve(box_args[0])) else {
        return None;
    };
    let res = ctx.resolver.resolve_member(raw_kind, name, args).ok()?;
    (protocol_dispatch || !kestrel_semantics::is_requirement(ctx.query_ctx, res.entity))
        .then_some(res)
}

/// The type an existential pins for an associated type: on the raw
/// existential by entity or name, and on its box by entity only (`Self` is
/// the box when a requirement is reached through `any P: P`). A box's own
/// associated types (`Indirection.Target`, looked up by name) are never
/// shadowed.
pub(crate) fn pinned_assoc_of(ctx: &InferCtx<'_>, kind: &TyKind, key: &AssocKey) -> Option<TyVar> {
    match kind {
        TyKind::RawExistential { bindings, .. } => pinned_assoc(ctx, bindings, key),
        TyKind::Struct { args, .. } | TyKind::Enum { args, .. } if matches!(key, AssocKey::Entity(_)) => {
            let [payload] = args.as_slice() else {
                return None;
            };
            match ctx.slot(ctx.resolve(*payload)) {
                TySlot::Resolved(TyKind::RawExistential { bindings, .. }) => {
                    pinned_assoc(ctx, bindings, key)
                },
                _ => None,
            }
        },
        _ => None,
    }
}

/// The type a raw existential pins for an associated type, if it pins it.
fn pinned_assoc(
    ctx: &InferCtx<'_>,
    bindings: &[(Entity, TyVar)],
    key: &AssocKey,
) -> Option<TyVar> {
    bindings
        .iter()
        .find(|(assoc, _)| match key {
            AssocKey::Entity(e) => e == assoc,
            AssocKey::Name(n) => crate::resolve::assoc_name(ctx.query_ctx, *assoc) == *n,
        })
        .map(|(_, ty)| *ty)
}
