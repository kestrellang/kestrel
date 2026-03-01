//! SelfProtocolBounds query - collect protocol IDs that Self is bounded by

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Constraint, TyKind};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{SymbolFor, WhereClausesInScope};
use crate::query::Query;

/// Collect protocol IDs that `Self` is bounded by in a given context.
///
/// Gathers bounds from:
/// 1. Where clauses with `Self: Protocol` constraints
/// 2. The enclosing protocol (if inside a protocol)
/// 3. The extension target protocol (if inside a protocol extension)
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SelfProtocolBounds {
    pub context_id: SymbolId,
}

impl Query for SelfProtocolBounds {
    type Output = Vec<SymbolId>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut result = Vec::new();

        // Self bounds from where clauses (Self: Protocol)
        let where_clauses = model.query(WhereClausesInScope {
            context_id: self.context_id,
        });
        for wc in where_clauses {
            for constraint in wc.constraints() {
                if let Constraint::SelfBound {
                    associated_type_path,
                    bounds,
                    ..
                } = constraint
                    && associated_type_path.is_empty()
                {
                    for bound in bounds {
                        if let TyKind::Protocol { symbol, .. } = bound.kind() {
                            result.push(symbol.metadata().id());
                        }
                    }
                }
            }
        }

        // Also add the enclosing protocol or protocol extension target, if any
        let mut current = Some(self.context_id);
        while let Some(id) = current {
            let Some(symbol) = model.query(SymbolFor { id }) else {
                break;
            };

            if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
                result.push(symbol.metadata().id());
                break;
            }

            if symbol.metadata().kind() == KestrelSymbolKind::Extension
                && let Some(target_beh) =
                    symbol.metadata().get_behavior::<ExtensionTargetBehavior>()
            {
                let target_ty = target_beh.target_type();
                if let TyKind::Protocol { symbol, .. } = target_ty.kind() {
                    result.push(symbol.metadata().id());
                    break;
                }
            }

            current = symbol.metadata().parent().map(|p| p.metadata().id());
        }

        result
    }
}
