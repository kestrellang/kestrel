//! AllConformancesFor query - all conformances from a symbol plus its extensions

use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ConformancesForSymbol, ExtensionsFor};
use crate::query::Query;

/// Collect all protocol conformances for a type, including conformances
/// declared in extensions.
pub struct AllConformancesFor {
    pub symbol_id: SymbolId,
}

impl Query for AllConformancesFor {
    type Output = Vec<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut conformances = model.query(ConformancesForSymbol {
            symbol_id: self.symbol_id,
        });

        let extensions = model.query(ExtensionsFor {
            target_id: self.symbol_id,
        });
        for extension in &extensions {
            let ext_confs = model.query(ConformancesForSymbol {
                symbol_id: extension.metadata().id(),
            });
            conformances.extend(ext_confs);
        }

        conformances
    }
}
