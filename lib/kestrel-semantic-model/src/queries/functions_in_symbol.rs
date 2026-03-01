//! FunctionsInSymbol query - collect function children for a symbol

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get function children (direct members) of a symbol.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct FunctionsInSymbol {
    pub parent_id: SymbolId,
}

impl Query for FunctionsInSymbol {
    type Output = Vec<Arc<FunctionSymbol>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let parent = match model.query(SymbolFor { id: self.parent_id }) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let parent_dyn: Arc<dyn Symbol<KestrelLanguage>> = parent;
        parent_dyn
            .metadata()
            .children()
            .into_iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
            .filter_map(|child| child.downcast_arc::<FunctionSymbol>().ok())
            .collect()
    }
}
