//! ImportsInScope query - get all imports in scope for a symbol

use std::sync::Arc;

use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;
use crate::scope::{Import, ImportItem};

/// Get all imports in scope for a symbol.
///
/// Returns a list of Import metadata extracted from ImportDataBehavior
/// on the symbol's children.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ImportsInScope {
    pub symbol_id: SymbolId,
}

impl Query for ImportsInScope {
    type Output = Vec<Arc<Import>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model
            .query(SymbolFor { id: self.symbol_id })
            .expect("symbol must exist");

        symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|c| matches!(c.metadata().kind(), KestrelSymbolKind::Import))
            .filter_map(|import_symbol| {
                import_symbol
                    .metadata()
                    .get_behavior::<ImportDataBehavior>()
                    .map(|data| {
                        Arc::new(Import {
                            module_path: data.module_path().to_vec(),
                            alias: data.alias().map(|s| s.to_string()),
                            items: data
                                .items()
                                .iter()
                                .map(|i| ImportItem {
                                    name: i.name.clone(),
                                    alias: i.alias.clone(),
                                    target_id: i.target_id,
                                })
                                .collect(),
                        })
                    })
            })
            .collect()
    }
}
