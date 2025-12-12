//! IsVisibleFrom query - check if a target symbol is visible from a context

use std::sync::Arc;

use kestrel_semantic_tree::behavior::visibility::Visibility;
use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::queries::{AncestorOfKind, SymbolFor};
use crate::query::Query;
use crate::SemanticModel;

/// Check if a target symbol is visible from a context.
///
/// Applies visibility rules (public, private, internal, fileprivate)
/// to determine if `target` can be accessed from `context`.
pub struct IsVisibleFrom {
    pub target: SymbolId,
    pub context: SymbolId,
}

impl Query for IsVisibleFrom {
    type Output = bool;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let target_symbol = match model.query(SymbolFor { id: self.target }) {
            Some(s) => s,
            None => return false,
        };
        let context_symbol = match model.query(SymbolFor { id: self.context }) {
            Some(s) => s,
            None => return false,
        };

        let Some(visibility_behavior) = target_symbol.visibility_behavior() else {
            // No visibility behavior means default (internal), which is always visible
            return true;
        };

        match visibility_behavior.visibility() {
            Some(Visibility::Public) => true,
            Some(Visibility::Private) => {
                let visibility_scope = visibility_behavior.visibility_scope();
                Arc::ptr_eq(&context_symbol, visibility_scope)
                    || is_ancestor(visibility_scope, &context_symbol)
            }
            Some(Visibility::Internal) => {
                let target_module = model.query(AncestorOfKind {
                    symbol_id: self.target,
                    kind: KestrelSymbolKind::Module,
                });
                let context_module = model.query(AncestorOfKind {
                    symbol_id: self.context,
                    kind: KestrelSymbolKind::Module,
                });

                match (target_module, context_module) {
                    (Some(t), Some(c)) => t == c,
                    _ => true, // If we can't determine modules, default to visible
                }
            }
            Some(Visibility::Fileprivate) => {
                let visibility_scope = visibility_behavior.visibility_scope();
                Arc::ptr_eq(&context_symbol, visibility_scope)
                    || is_ancestor(visibility_scope, &context_symbol)
            }
            None => true, // Default visibility (internal) - visible everywhere
        }
    }
}

/// Check if potential_ancestor is an ancestor of the given symbol.
fn is_ancestor(
    potential_ancestor: &Arc<dyn Symbol<KestrelLanguage>>,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> bool {
    let mut current = Some(symbol.clone());

    while let Some(s) = current {
        if Arc::ptr_eq(&s, potential_ancestor) {
            return true;
        }
        current = s.metadata().parent();
    }

    false
}
