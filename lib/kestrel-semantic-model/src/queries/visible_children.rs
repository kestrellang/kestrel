//! VisibleChildren query - get visible children of a symbol from a context

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{IsVisibleFrom, SymbolFor};
use crate::query::Query;

/// Get visible children of a symbol from a given context.
///
/// Returns all children of `parent` that are visible when accessed
/// from `context`, applying visibility rules.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct VisibleChildren {
    pub parent: SymbolId,
    pub context: SymbolId,
}

impl Query for VisibleChildren {
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
            .filter(|child| {
                let child_id = child.metadata().id();
                model.query(IsVisibleFrom {
                    target: child_id,
                    context: self.context,
                })
            })
            .collect()
    }
}
