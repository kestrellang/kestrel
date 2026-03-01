//! AllMethodsFor query - all methods from a symbol plus its extensions

use std::sync::Arc;

use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, FunctionsInSymbol};
use crate::query::Query;

/// Collect all function methods for a type, including methods
/// declared in extensions.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AllMethodsFor {
    pub symbol_id: SymbolId,
}

impl Query for AllMethodsFor {
    type Output = Vec<Arc<FunctionSymbol>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut methods = model.query(FunctionsInSymbol {
            parent_id: self.symbol_id,
        });

        let extensions = model.query(ExtensionsFor {
            target_id: self.symbol_id,
        });
        for extension in &extensions {
            let ext_methods = model.query(FunctionsInSymbol {
                parent_id: extension.metadata().id(),
            });
            methods.extend(ext_methods);
        }

        methods
    }
}
