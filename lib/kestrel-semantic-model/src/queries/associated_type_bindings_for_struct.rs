//! AssociatedTypeBindingsForStruct query - map type aliases to associated-type bindings for a struct

use std::collections::HashMap;

use kestrel_semantic_tree::behavior::callable::SignatureType;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ResolvedAliasedType, SymbolFor};
use crate::query::Query;

/// Collect type-alias bindings within a struct that can satisfy protocol associated types.
///
/// This currently looks at `typealias Name = <type>` inside the struct body.
pub struct AssociatedTypeBindingsForStruct {
    pub struct_id: SymbolId,
}

impl Query for AssociatedTypeBindingsForStruct {
    type Output = HashMap<String, SignatureType>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor { id: self.struct_id }) else {
            return HashMap::new();
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return HashMap::new();
        }

        let struct_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = symbol;
        let mut bindings = HashMap::new();
        for child in struct_dyn.metadata().children() {
            if child.metadata().kind() != KestrelSymbolKind::TypeAlias {
                continue;
            }
            let Ok(type_alias) = child.downcast_arc::<TypeAliasSymbol>() else {
                continue;
            };
            let name = type_alias.metadata().name().value.clone();
            let alias_id = type_alias.metadata().id();
            let Some(resolved) = model.query(ResolvedAliasedType {
                type_alias_id: alias_id,
            }) else {
                continue;
            };
            bindings.insert(name, SignatureType::from_ty(&resolved));
        }
        bindings
    }
}
