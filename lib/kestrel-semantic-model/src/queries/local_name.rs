//! LocalName query - map a container + LocalId to a user-facing name

use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::local::LocalId;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Look up a local's name within a specific container (function or initializer).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LocalName {
    pub container_id: SymbolId,
    pub local_id: LocalId,
}

impl Query for LocalName {
    type Output = Option<String>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let container = model.query(SymbolFor {
            id: self.container_id,
        })?;

        if let Ok(func) = container.clone().downcast_arc::<FunctionSymbol>() {
            return func.get_local(self.local_id).map(|l| l.name().to_string());
        }

        if let Ok(init) = container.downcast_arc::<InitializerSymbol>() {
            return init.get_local(self.local_id).map(|l| l.name().to_string());
        }

        None
    }
}
