//! AssociatedTypeBindingsForEnum query - map type aliases to associated-type bindings for an enum

use std::collections::HashMap;

use kestrel_semantic_tree::behavior::callable::SignatureType;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, ResolvedAliasedType, SymbolFor};
use crate::query::Query;

/// Collect type-alias bindings within an enum that can satisfy protocol associated types.
///
/// This currently looks at `typealias Name = <type>` inside the enum body.
pub struct AssociatedTypeBindingsForEnum {
    pub enum_id: SymbolId,
}

impl Query for AssociatedTypeBindingsForEnum {
    type Output = HashMap<String, SignatureType>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor { id: self.enum_id }) else {
            return HashMap::new();
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Enum {
            return HashMap::new();
        }

        let enum_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = symbol;
        let mut bindings = HashMap::new();

        let mut collect_from_symbol = |parent: &std::sync::Arc<dyn Symbol<KestrelLanguage>>| {
            for child in parent.metadata().children() {
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
        };

        // 1) Bindings in the enum body
        collect_from_symbol(&enum_dyn);

        // 2) Bindings declared in extensions of this enum
        let extensions = model.query(ExtensionsFor {
            target_id: self.enum_id,
        });
        for ext in &extensions {
            let ext_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = ext.clone();
            collect_from_symbol(&ext_dyn);
        }

        bindings
    }
}
