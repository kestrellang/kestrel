//! ScopeFor query - get the scope for a symbol

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::queries::SymbolFor;
use crate::query::Query;
use crate::scope::Scope;
use crate::SemanticModel;

/// Get the scope for a symbol.
///
/// The scope contains:
/// - `symbol_id`: The symbol this scope belongs to
/// - `imports`: Empty for now (import resolution happens in bind phase)
/// - `declarations`: Non-import children mapped by name
/// - `parent`: Parent symbol's ID for scope chain lookup
pub struct ScopeFor {
    pub symbol_id: SymbolId,
}

impl Query for ScopeFor {
    type Output = Arc<Scope>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model
            .query(SymbolFor { id: self.symbol_id })
            .expect("symbol must exist");

        // Get declarations (children that aren't imports)
        let declarations = symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|c| !matches!(c.metadata().kind(), KestrelSymbolKind::Import))
            .fold(HashMap::new(), |mut map, child| {
                map.entry(child.metadata().name().value.clone())
                    .or_insert_with(Vec::new)
                    .push(child.metadata().id());
                map
            });

        Arc::new(Scope {
            symbol_id: self.symbol_id,
            imports: HashMap::new(), // Imports are resolved during bind phase
            declarations,
            parent: symbol.metadata().parent().map(|p| p.metadata().id()),
        })
    }
}
