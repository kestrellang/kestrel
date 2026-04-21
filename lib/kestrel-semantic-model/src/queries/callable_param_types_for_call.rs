//! callable_param_types_for_call - compute expected parameter types for a call-like expression

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::expr::{CallArgument, ExprKind, Expression};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::SemanticModel;
use crate::queries::{StructFields, SymbolFor};
use crate::resolve_all_associated_types;

/// Get the expected parameter types for a call-like expression.
///
/// Supports:
/// - `ExprKind::Call` with `SymbolRef` or `MethodRef` callee
/// - `ExprKind::ImplicitStructInit` (memberwise initializer)
///
/// This is a plain function (not a query) because it borrows `&Expression`
/// and cannot satisfy the `Hash + Eq + Clone + 'static` bounds required for memoization.
pub fn callable_param_types_for_call(expr: &Expression, model: &SemanticModel) -> Option<Vec<Ty>> {
    match &expr.kind {
        ExprKind::Call {
            callee,
            substitutions,
            arguments,
            ..
        } => {
            // IMPORTANT: Prefer using the callee's resolved type if it's a function type.
            // The body resolver already computed fully resolved parameter types (including
            // associated type resolution) and stored them in callee.ty. Re-querying the
            // symbol would give us unresolved types with associated types like
            // ArrayIterator[T].Item instead of the resolved type T.
            if let TyKind::Function { params, .. } = callee.ty.kind() {
                return Some(params.clone());
            }

            // Fallback: re-query the symbol if callee type is not a resolved function
            match &callee.kind {
                ExprKind::SymbolRef(symbol_id) => {
                    let symbol = model.query(SymbolFor { id: *symbol_id })?;
                    let callable = symbol.metadata().get_behavior::<CallableBehavior>()?;
                    Some(
                        callable
                            .parameters()
                            .iter()
                            .map(|p| {
                                let ty = p.ty.apply_substitutions(substitutions);
                                resolve_all_associated_types(model, &ty)
                            })
                            .collect(),
                    )
                },
                ExprKind::MethodRef {
                    candidates,
                    receiver,
                    ..
                } => best_method_param_types_for_call(
                    model,
                    candidates,
                    &receiver.ty,
                    substitutions,
                    arguments,
                ),
                _ => None,
            }
        },
        ExprKind::ImplicitStructInit { .. } => {
            let (struct_sym, substitutions) = expr.ty.as_struct_with_subs()?;
            let struct_id = struct_sym.metadata().id();
            let fields = model.query(StructFields { struct_id });
            Some(
                fields
                    .into_iter()
                    .map(|field| field.ty.apply_substitutions(substitutions))
                    .collect(),
            )
        },
        _ => None,
    }
}

fn best_method_param_types_for_call(
    model: &SemanticModel,
    candidates: &[semantic_tree::symbol::SymbolId],
    receiver_ty: &Ty,
    substitutions: &kestrel_semantic_tree::ty::Substitutions,
    arguments: &[CallArgument],
) -> Option<Vec<Ty>> {
    let mut best: Option<(usize, Vec<Ty>)> = None;

    for &id in candidates {
        let Some(symbol) = model.query(SymbolFor { id }) else {
            continue;
        };
        let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() else {
            continue;
        };

        let params: Vec<Ty> = callable
            .parameters()
            .iter()
            .map(|p| {
                let ty = p.ty.apply_substitutions(substitutions);
                let ty = ty.substitute_self(receiver_ty);
                resolve_all_associated_types(model, &ty)
            })
            .collect();

        if params.len() != arguments.len() {
            continue;
        }

        // Prefer candidate with the most argument type matches.
        let mut score = 0usize;
        for (arg, param_ty) in arguments.iter().zip(params.iter()) {
            if arg.value.ty.to_string() == param_ty.to_string() {
                score += 2;
            }
            if arg.label.as_ref().is_some_and(|label| {
                callable
                    .parameters()
                    .iter()
                    .any(|p| p.label.as_ref().is_some_and(|pl| pl.value == *label))
            }) {
                score += 1;
            }
        }

        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, params));
        }
    }

    best.map(|(_, params)| params)
}
