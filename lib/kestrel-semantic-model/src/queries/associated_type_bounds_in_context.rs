//! AssociatedTypeBoundsInContext query - collect protocol bounds on an associated type

use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::ty::{Constraint, Ty, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{SymbolFor, WhereClausesInScope};
use crate::query::Query;

/// Collect all protocol bounds on an associated type from:
/// 1. Direct bounds on the associated type declaration
/// 2. Where clause constraints in scope (SelfBound, InheritedAssociatedTypeBound, TypeBound)
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssociatedTypeBoundsInContext {
    pub assoc_type_id: SymbolId,
    pub context_id: Option<SymbolId>,
}

impl Query for AssociatedTypeBoundsInContext {
    type Output = Vec<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor { id: self.assoc_type_id }) else {
            return Vec::new();
        };

        let Ok(assoc_type) = symbol.downcast_arc::<AssociatedTypeSymbol>() else {
            return Vec::new();
        };

        let mut bounds = Vec::new();

        // Collect direct bounds from the associated type declaration
        if let Some(direct_bounds) = assoc_type.bounds() {
            for bound in direct_bounds {
                if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                    bounds.push(bound.clone());
                }
            }
        }

        let Some(context_id) = self.context_id else {
            return bounds;
        };

        // Collect bounds from where clauses in scope
        let assoc_name = assoc_type.metadata().name().value.clone();
        let where_clauses = model.query(WhereClausesInScope { context_id });
        for wc in where_clauses {
            for constraint in wc.constraints() {
                if let Constraint::SelfBound {
                    associated_type_path,
                    bounds: self_bounds,
                    ..
                } = constraint
                    && !associated_type_path.is_empty()
                    && associated_type_path.last() == Some(&assoc_name)
                {
                    for bound in self_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
                if let Constraint::InheritedAssociatedTypeBound {
                    path,
                    bounds: assoc_bounds,
                    ..
                } = constraint
                    && path.split('.').next_back() == Some(assoc_name.as_str())
                {
                    for bound in assoc_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
                if let Constraint::TypeBound {
                    param: None,
                    param_name,
                    bounds: param_bounds,
                    ..
                } = constraint
                    && param_name == &assoc_name
                {
                    for bound in param_bounds {
                        if matches!(bound.kind(), TyKind::Protocol { .. } | TyKind::Error) {
                            bounds.push(bound.clone());
                        }
                    }
                }
            }
        }

        bounds
    }
}
