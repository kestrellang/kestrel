//! ExtensionMethods query - collect method names/spans for an extension symbol

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get all function children (method declarations) of an extension.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionMethods {
    pub extension_id: SymbolId,
}

impl Query for ExtensionMethods {
    type Output = Vec<(String, Span)>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let extension_symbol = match model.query(SymbolFor {
            id: self.extension_id,
        }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        if extension_symbol.metadata().kind() != KestrelSymbolKind::Extension {
            return Vec::new();
        }

        let extension_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = extension_symbol;
        extension_dyn
            .metadata()
            .children()
            .into_iter()
            .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
            .map(|child| {
                (
                    child.metadata().name().value.clone(),
                    child.metadata().name().span.clone(),
                )
            })
            .collect()
    }
}
