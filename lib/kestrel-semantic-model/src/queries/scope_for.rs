//! ScopeFor query - get the scope for a symbol

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::symbol::import::ImportDataBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;
use crate::scope::Scope;

/// Get the scope for a symbol.
///
/// The scope contains:
/// - `symbol_id`: The symbol this scope belongs to
/// - `imports`: Resolved imports from Import children
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

        let visible = symbol.metadata().visible_children();

        // Collect resolved imports from Import children
        let mut imports: HashMap<String, Vec<SymbolId>> = HashMap::new();
        for child in &visible {
            if child.metadata().kind() == KestrelSymbolKind::Import {
                if let Some(import_data) = child.metadata().get_behavior::<ImportDataBehavior>() {
                    // Collect each resolved import item
                    for item in import_data.items() {
                        if let Some(target_id) = item.target_id {
                            // Use alias if present, otherwise use original name
                            let name = item.alias.clone().unwrap_or_else(|| item.name.clone());
                            imports.entry(name).or_default().push(target_id);
                        }
                    }
                }
            }
        }

        // Get declarations (children that aren't imports)
        // Use visible_children() to flatten through transparent symbols like SourceFile,
        // so declarations from all files in a module are visible to each other.
        let declarations = visible
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
            imports,
            declarations,
            parent: symbol.metadata().parent().map(|p| p.metadata().id()),
        })
    }
}
