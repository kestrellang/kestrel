//! StructMethods query - collect method names/spans for a struct symbol

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get all function children (method declarations) of a struct.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StructMethods {
    pub struct_id: SymbolId,
}

impl Query for StructMethods {
    type Output = Vec<(String, Span)>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let struct_symbol = match model.query(SymbolFor { id: self.struct_id }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        if struct_symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return Vec::new();
        }

        let struct_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = struct_symbol;
        struct_dyn
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
