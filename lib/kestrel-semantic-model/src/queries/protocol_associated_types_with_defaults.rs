//! ProtocolAssociatedTypesWithDefaults query - collect associated types declared by a protocol

use std::collections::HashMap;

use kestrel_semantic_tree::behavior::callable::SignatureType;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Collect associated types declared by a protocol, along with their default types (if any).
pub struct ProtocolAssociatedTypesWithDefaults {
    pub protocol_id: SymbolId,
}

impl Query for ProtocolAssociatedTypesWithDefaults {
    type Output = HashMap<String, Option<SignatureType>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor {
            id: self.protocol_id,
        }) else {
            return HashMap::new();
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return HashMap::new();
        }

        let protocol_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = symbol;
        let mut associated_types = HashMap::new();
        for child in protocol_dyn.metadata().children() {
            if child.metadata().kind() != KestrelSymbolKind::AssociatedType {
                continue;
            }
            let Ok(assoc_type) = child.downcast_arc::<AssociatedTypeSymbol>() else {
                continue;
            };
            let name = assoc_type.metadata().name().value.clone();
            let default_type = assoc_type
                .default_type()
                .map(|ty| SignatureType::from_ty(&ty));
            associated_types.insert(name, default_type);
        }
        associated_types
    }
}
