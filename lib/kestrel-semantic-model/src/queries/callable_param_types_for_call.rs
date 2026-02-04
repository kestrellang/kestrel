//! CallableParamTypesForCall query - compute expected parameter types for a call-like expression

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::SemanticModel;
use crate::queries::{StructFields, SymbolFor};
use crate::query::Query;

/// Get the expected parameter types for a call-like expression.
///
/// Supports:
/// - `ExprKind::Call` with `SymbolRef` or `MethodRef` callee
/// - `ExprKind::ImplicitStructInit` (memberwise initializer)
pub struct CallableParamTypesForCall<'a> {
    pub expr: &'a Expression,
}

impl<'a> Query for CallableParamTypesForCall<'a> {
    type Output = Option<Vec<Ty>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        match &self.expr.kind {
            ExprKind::Call {
                callee,
                substitutions,
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
                                .map(|p| p.ty.apply_substitutions(substitutions))
                                .collect(),
                        )
                    },
                    ExprKind::MethodRef {
                        candidates,
                        receiver,
                        ..
                    } => {
                        for &id in candidates {
                            let Some(symbol) = model.query(SymbolFor { id }) else {
                                continue;
                            };
                            let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>()
                            else {
                                continue;
                            };
                            return Some(
                                callable
                                    .parameters()
                                    .iter()
                                    .map(|p| {
                                        let ty = p.ty.apply_substitutions(substitutions);
                                        ty.substitute_self(&receiver.ty)
                                    })
                                    .collect(),
                            );
                        }
                        None
                    },
                    _ => None,
                }
            },
            ExprKind::ImplicitStructInit { .. } => {
                let (struct_sym, substitutions) = self.expr.ty.as_struct_with_subs()?;
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
}
