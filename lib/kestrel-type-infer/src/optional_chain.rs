//! `?.` mode decision (optional-chaining design §2).
//!
//! HIR lowering gives every bind a provisional method: `chainTake` for an
//! rvalue base, `chainBorrow` for a place base. A place base must call
//! `chainMutate` instead when the rest of the chain mutates the payload
//! (`a?.push(x)`, `a?.n = 1`, `a?.n += 1`), which is only known once members
//! resolve. This pass runs after solving and before the result is built: it
//! finds every borrowed bind whose binder `$o` is the root of a mutating use,
//! upgrades it to `ChainMode::Mutate`, and flips the binder's reference type
//! to `&mutating` (Q3: the solve used `&W` throughout, and `Ref` mutability is
//! not a typing question).
//!
//! The result, `chain_modes`, is the single source of truth: MIR lowering
//! calls the method it names and `access_mode` checks the base against it.

use kestrel_ast_builder::{Callable, Name, ReceiverKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::{HirBody, HirCallArg, HirExpr, HirExprId};
use kestrel_hir::res::LocalId;

use crate::collections::HashMap;
use crate::ctx::InferCtx;
use crate::result::ChainMode;
use crate::ty::{TyKind, TySlot};

/// Decide every bind's mode and record it in `ctx.chain_modes`.
pub(crate) fn decide_chain_modes(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    let binds = hir.optional_chain_binds();
    if binds.is_empty() {
        return;
    }
    for bind in &binds {
        let mode = match &hir.exprs[bind.scrutinee] {
            HirExpr::ProtocolCall { method, .. } if method.as_str_or_empty() == "chainTake" => {
                ChainMode::Take
            },
            _ => ChainMode::Borrow,
        };
        ctx.chain_modes.insert(bind.scrutinee, mode);
    }

    // Mutation propagates outward (`a?.b?.push(x)` mutates `a` too): a bind
    // whose base is rooted at an outer binder makes that binder mutated once
    // it is itself `Mutate`. Iterate to a fixpoint; each round only upgrades.
    let by_binder: HashMap<LocalId, HirExprId> =
        binds.iter().map(|b| (b.binder, b.scrutinee)).collect();
    loop {
        let mutated = mutated_binders(ctx, hir, &by_binder);
        let mut changed = false;
        for binder in mutated {
            let Some(&scrutinee) = by_binder.get(&binder) else {
                continue;
            };
            if ctx.chain_modes.get(&scrutinee) != Some(&ChainMode::Borrow) {
                continue;
            }
            ctx.chain_modes.insert(scrutinee, ChainMode::Mutate);
            make_binder_mutating(ctx, binder);
            changed = true;
        }
        if !changed {
            break;
        }
    }
}

/// Binders that are the root of a mutating use somewhere in the body.
fn mutated_binders(
    ctx: &InferCtx<'_>,
    hir: &HirBody,
    by_binder: &HashMap<LocalId, HirExprId>,
) -> Vec<LocalId> {
    let mut out = Vec::new();
    let mark = |place: HirExprId, out: &mut Vec<LocalId>| {
        if let Some(local) = hir.place_root(place)
            && by_binder.contains_key(&local)
        {
            out.push(local);
        }
    };
    for (expr_id, expr) in hir.exprs.iter() {
        match expr {
            HirExpr::Assign { target, .. } => mark(*target, &mut out),
            HirExpr::Borrow {
                inner,
                mutating: true,
                ..
            } => mark(*inner, &mut out),
            HirExpr::MethodCall { receiver, args, .. } => {
                let Some(&callee) = ctx.resolutions.get(&expr_id) else {
                    continue;
                };
                mutating_uses(ctx, callee, Some(*receiver), args, |p| mark(p, &mut out));
            },
            HirExpr::Call { callee, args, .. } => {
                let entity = match &hir.exprs[*callee] {
                    HirExpr::Def(entity, _, _) => Some(*entity),
                    _ => ctx.resolutions.get(callee).copied(),
                };
                if let Some(entity) = entity {
                    mutating_uses(ctx, entity, None, args, |p| mark(p, &mut out));
                }
            },
            HirExpr::ProtocolCall {
                receiver,
                protocol,
                method,
                args,
                ..
            } => {
                // A nested bind already upgraded mutates its base.
                if ctx.chain_modes.get(&expr_id) == Some(&ChainMode::Mutate) {
                    mark(*receiver, &mut out);
                    continue;
                }
                if let Some(entity) = protocol_method(ctx, *protocol, method.as_str_or_empty()) {
                    mutating_uses(ctx, entity, Some(*receiver), args, |p| mark(p, &mut out));
                }
            },
            _ => {},
        }
    }
    out
}

/// Call `mark` on the receiver of a `mutating` method and on each argument
/// passed to a `mutating` (not consuming) parameter — the same rule as the
/// `access_mode` analyzer.
fn mutating_uses(
    ctx: &InferCtx<'_>,
    callee: Entity,
    receiver: Option<HirExprId>,
    args: &[HirCallArg],
    mut mark: impl FnMut(HirExprId),
) {
    let Some(callable) = ctx.query_ctx.get::<Callable>(callee) else {
        return;
    };
    if let Some(receiver) = receiver
        && matches!(callable.receiver, Some(ReceiverKind::Mutating))
    {
        mark(receiver);
    }
    for (arg, param) in args.iter().zip(callable.params.iter()) {
        if param.is_mut && !param.is_consuming {
            mark(arg.value);
        }
    }
}

/// The protocol's requirement named `name`.
fn protocol_method(ctx: &InferCtx<'_>, protocol: Entity, name: &str) -> Option<Entity> {
    ctx.query_ctx
        .children_of(protocol)
        .iter()
        .copied()
        .find(|&child| {
            ctx.query_ctx
                .get::<Name>(child)
                .is_some_and(|n| n.0 == name)
                && ctx.query_ctx.get::<Callable>(child).is_some()
        })
}

/// Retype a mutated binder from `&W` to `&mutating W`. Every use of the
/// binder (and the bind's `Optional[&W]` result) shares this `Ref` slot.
fn make_binder_mutating(ctx: &mut InferCtx<'_>, binder: LocalId) {
    let Some(&tv) = ctx.local_types.get(&binder) else {
        return;
    };
    let rep = ctx.resolve(tv);
    if let TySlot::Resolved(TyKind::Ref { mutating, .. }) = &mut ctx.types[rep.0 as usize] {
        *mutating = true;
    }
}
