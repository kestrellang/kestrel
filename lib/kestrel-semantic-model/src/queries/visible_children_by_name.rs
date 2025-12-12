//! VisibleChildrenByName query - find visible children by name

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::queries::{IsVisibleFrom, SymbolFor};
use crate::query::Query;
use crate::SemanticModel;

/// Find children of parent that are visible from context and match name.
///
/// Combines symbol lookup with visibility checking and name filtering.
pub struct VisibleChildrenByName {
    pub parent: SymbolId,
    pub name: String,
    pub context: SymbolId,
}

impl Query for VisibleChildrenByName {
    type Output = Vec<Arc<dyn Symbol<KestrelLanguage>>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let parent_symbol = match model.query(SymbolFor { id: self.parent }) {
            Some(s) => s,
            None => return Vec::new(),
        };

        parent_symbol
            .metadata()
            .visible_children()
            .into_iter()
            .filter(|child| child.metadata().name().value == self.name)
            .filter(|child| {
                model.query(IsVisibleFrom {
                    target: child.metadata().id(),
                    context: self.context,
                })
            })
            .collect()
    }
}
