//! Which operands an expression uses mutably — the one definition shared by
//! closure capture planning (a mutated capture is a `Write` capture, which is
//! what E603 rejects in a normal closure) and the access-mode analyzer
//! (E203–E207: a mutated operand must be a mutable place).
//!
//! A write is not only `x = v`. A call mutates its receiver when the method is
//! `mutating` (`xs.append(1)`, and `x += 1`, which desugars to a `mutating`
//! protocol requirement), an argument passed to a `mutating` parameter, and
//! the base of a `modifying` member read. Two hand-written walks that each
//! knew only `Assign` let all three slip past E603.

use kestrel_ast_builder::{Callable, MutatingRead, Name, NodeKind, ReceiverKind};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};

use crate::result::{ResolvedTy, TypedBody};

/// How an operand is used mutably.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MutatingOperand {
    /// The receiver of a `mutating` method. A temporary receiver is fine
    /// (the call owns it); a named place must be mutable.
    Receiver,
    /// The base of a `modifying` field or subscript read (`c.next`, `c(i)`).
    ModifyingBase,
    /// An argument to a `mutating` parameter. It must be a mutable place.
    Arg,
}

/// The function a call expression dispatches to: the callee of a `Call`, the
/// resolved method of a `MethodCall`, or the requirement of a `ProtocolCall`.
pub fn call_target(
    world: &QueryContext<'_>,
    typed: &TypedBody,
    hir: &HirBody,
    root: Entity,
    expr: HirExprId,
) -> Option<Entity> {
    match &hir.exprs[expr] {
        HirExpr::Call { callee, .. } => match &hir.exprs[*callee] {
            HirExpr::Def(entity, _, _) => Some(*entity),
            _ => typed.resolutions.get(callee).copied(),
        },
        HirExpr::MethodCall { .. } => typed.resolutions.get(&expr).copied(),
        HirExpr::ProtocolCall { .. } => protocol_call_method(world, typed, hir, root, expr),
        _ => None,
    }
}

/// The requirement a `ProtocolCall` (`expr`) dispatches through. A `?.`
/// bind's is the mode inference settled on; a tagged desugaring names its
/// own; only an untagged call (no stdlib tag) is matched by written name AND
/// call-site labels.
pub fn protocol_call_method(
    world: &QueryContext<'_>,
    typed: &TypedBody,
    hir: &HirBody,
    root: Entity,
    expr: HirExprId,
) -> Option<Entity> {
    let HirExpr::ProtocolCall {
        protocol,
        method,
        requirement,
        args,
        ..
    } = &hir.exprs[expr]
    else {
        return None;
    };
    if let Some(builtin) = typed.chain_requirement(expr) {
        return world.query(kestrel_name_res::ResolveBuiltin { builtin, root });
    }
    if let Some(requirement) = requirement {
        return Some(*requirement);
    }
    let name = method.as_str_or_empty();
    let labels: Vec<Option<String>> = args.iter().map(|a| a.label.clone()).collect();
    world.children_of(*protocol).iter().copied().find(|&c| {
        world.get::<NodeKind>(c) == Some(&NodeKind::Function)
            && world.get::<Name>(c).is_some_and(|n| n.0 == name)
            && world.get::<Callable>(c).is_some_and(|callable| {
                callable
                    .params
                    .iter()
                    .map(|p| p.label.clone())
                    .eq(labels.iter().cloned())
            })
    })
}

/// The operands `expr` uses mutably, with how. Empty for anything that isn't
/// a call or a `modifying` member read.
pub fn mutating_operands(
    world: &QueryContext<'_>,
    typed: &TypedBody,
    hir: &HirBody,
    root: Entity,
    expr: HirExprId,
) -> Vec<(HirExprId, MutatingOperand)> {
    let mut out = Vec::new();
    let (receiver, args) = match &hir.exprs[expr] {
        HirExpr::Call { callee, args, .. } => {
            // A read of a `modifying` subscript (`c(i)`) mutates its base.
            if is_modifying_read(world, typed, expr) {
                out.push((*callee, MutatingOperand::ModifyingBase));
            }
            (None, args)
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => (Some(*receiver), args),
        // A read of a `modifying` member (`c.next`) mutates its base.
        HirExpr::Field { base, .. } => {
            if is_modifying_read(world, typed, expr) {
                out.push((*base, MutatingOperand::ModifyingBase));
            }
            return out;
        },
        _ => return out,
    };
    let Some(target) = call_target(world, typed, hir, root, expr) else {
        return out;
    };
    // A method-call-shaped `modifying` read (`x.items(i)`) has a `mutating`
    // receiver on its `Callable`, so the receiver check below covers it.
    let Some(callable) = world.get::<Callable>(target) else {
        return out;
    };
    if let Some(recv) = receiver
        && matches!(callable.receiver, Some(ReceiverKind::Mutating))
    {
        out.push((recv, MutatingOperand::Receiver));
    }
    for (param, arg) in callable.params.iter().zip(args) {
        // A `mutating` closure-typed parameter means the callee calls the
        // closure exclusively, not that it writes the caller's binding: the
        // mutable place is the closure's own environment (E625's pairing).
        let closure_value = matches!(
            typed.expr_types.get(&arg.value),
            Some(ResolvedTy::Function { .. })
        );
        if param.is_mut && !param.is_consuming && !closure_value {
            out.push((arg.value, MutatingOperand::Arg));
        }
    }
    out
}

/// Is `expr` a read of a `modifying` field or subscript (one whose getter
/// mutates its receiver)?
pub fn is_modifying_read(world: &QueryContext<'_>, typed: &TypedBody, expr: HirExprId) -> bool {
    typed.resolutions.get(&expr).is_some_and(|&member| {
        world.has::<MutatingRead>(member)
            && matches!(
                world.get::<NodeKind>(member),
                Some(NodeKind::Field | NodeKind::Subscript)
            )
    })
}
