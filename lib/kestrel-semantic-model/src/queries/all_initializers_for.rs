//! AllInitializersFor query - all initializers from a symbol plus its extensions

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, SymbolFor};
use crate::query::Query;

/// Collect all initializers for a type, including initializers
/// declared in extensions.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AllInitializersFor {
    pub symbol_id: SymbolId,
}

impl Query for AllInitializersFor {
    type Output = Vec<Arc<InitializerSymbol>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut initializers = collect_initializers(model, self.symbol_id);

        let extensions = model.query(ExtensionsFor {
            target_id: self.symbol_id,
        });
        for extension in &extensions {
            let ext_inits = collect_initializers(model, extension.metadata().id());
            initializers.extend(ext_inits);
        }

        initializers
    }
}

fn collect_initializers(model: &SemanticModel, parent_id: SymbolId) -> Vec<Arc<InitializerSymbol>> {
    let Some(parent) = model.query(SymbolFor { id: parent_id }) else {
        return Vec::new();
    };

    let parent_dyn: Arc<dyn Symbol<KestrelLanguage>> = parent;
    parent_dyn
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Initializer)
        .filter_map(|child| child.downcast_arc::<InitializerSymbol>().ok())
        .collect()
}
