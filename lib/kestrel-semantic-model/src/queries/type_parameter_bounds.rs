//! TypeParameterBounds query - collect protocol bounds for a type parameter

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, SymbolFor};
use crate::query::Query;

/// Collect all protocol bounds on a type parameter by walking the parent chain.
///
/// Gathers bounds from where clauses at each enclosing scope, plus from
/// extensions on each enclosing type. Returns only `Protocol` and `Error` bounds.
pub struct TypeParameterBounds {
    pub param_id: SymbolId,
}

impl Query for TypeParameterBounds {
    type Output = Vec<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(param_symbol) = model.query(SymbolFor { id: self.param_id }) else {
            return Vec::new();
        };

        let mut bounds = Vec::new();
        let mut current = param_symbol.metadata().parent();

        while let Some(parent) = current {
            // Collect bounds from this parent's where clause
            if let Some(generics_beh) = parent.metadata().get_behavior::<GenericsBehavior>() {
                let wc = generics_beh.where_clause();
                for bound in wc.bounds_for(self.param_id) {
                    match bound.kind() {
                        TyKind::Protocol { .. } | TyKind::Error => {
                            bounds.push(bound.clone());
                        }
                        _ => {}
                    }
                }
            }

            // Also check extensions on this parent for additional bounds.
            // e.g., `extend Box[T] where T: Formattable` adds T: Formattable
            let parent_id = parent.metadata().id();
            let extensions = model.query(ExtensionsFor {
                target_id: parent_id,
            });
            for ext in &extensions {
                if let Some(target_beh) =
                    ext.metadata().get_behavior::<ExtensionTargetBehavior>()
                {
                    let ext_where = target_beh.where_clause();
                    for bound in ext_where.bounds_for(self.param_id) {
                        match bound.kind() {
                            TyKind::Protocol { .. } | TyKind::Error => {
                                bounds.push(bound.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }

            current = parent.metadata().parent();
        }

        bounds
    }
}
