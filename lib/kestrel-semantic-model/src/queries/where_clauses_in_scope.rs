//! WhereClausesInScope query - collect all where clauses by walking the parent chain

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::ty::WhereClause;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Collect all where clauses from a context by walking up the parent chain.
///
/// Gathers where clauses from both `GenericsBehavior` (on generic declarations)
/// and `ExtensionTargetBehavior` (on extensions).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct WhereClausesInScope {
    pub context_id: SymbolId,
}

impl Query for WhereClausesInScope {
    type Output = Vec<WhereClause>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut clauses = Vec::new();
        let mut current_id = Some(self.context_id);

        while let Some(id) = current_id {
            let Some(symbol) = model.query(SymbolFor { id }) else {
                break;
            };

            if let Some(generics_beh) = symbol.metadata().get_behavior::<GenericsBehavior>() {
                let wc = generics_beh.where_clause();
                if !wc.is_empty() {
                    clauses.push(wc.clone());
                }
            }

            if let Some(target_beh) =
                symbol.metadata().get_behavior::<ExtensionTargetBehavior>()
            {
                let wc = target_beh.where_clause();
                if !wc.is_empty() {
                    clauses.push(wc.clone());
                }
            }

            current_id = symbol.metadata().parent().map(|p| p.metadata().id());
        }

        clauses
    }
}
