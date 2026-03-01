//! ChildByName query - find a child symbol by name

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Find a child symbol by name (without visibility check).
///
/// Searches the visible children of the parent symbol for one
/// matching the given name. Does not perform visibility checking.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ChildByName {
    pub parent: SymbolId,
    pub name: String,
}

impl Query for ChildByName {
    type Output = Option<Arc<dyn Symbol<KestrelLanguage>>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let parent_symbol = model.query(SymbolFor { id: self.parent })?;

        parent_symbol
            .metadata()
            .visible_children()
            .into_iter()
            .find(|child| child.metadata().name().value == self.name)
    }
}
