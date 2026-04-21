//! AssociatedTypeBindingsFor query - unified type alias bindings for structs and enums

use std::collections::HashMap;
use std::sync::Arc;

use kestrel_semantic_tree::behavior::ConcreteTypeMarker;
use kestrel_semantic_tree::behavior::callable::SignatureType;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::conforms_to::ConformsToBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, ResolvedAliasedType, SymbolFor};
use crate::query::Query;

/// Collect type-alias bindings within a struct or enum that can satisfy protocol
/// associated types.
///
/// Unifies `AssociatedTypeBindingsForStruct` and `AssociatedTypeBindingsForEnum`.
/// Dispatches based on symbol kind internally.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AssociatedTypeBindingsFor {
    pub symbol_id: SymbolId,
}

impl Query for AssociatedTypeBindingsFor {
    type Output = HashMap<String, SignatureType>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor { id: self.symbol_id }) else {
            return HashMap::new();
        };

        if symbol
            .metadata()
            .get_behavior::<ConcreteTypeMarker>()
            .is_none()
        {
            return HashMap::new();
        }

        let symbol_dyn: Arc<dyn Symbol<KestrelLanguage>> = symbol;
        let mut bindings = HashMap::new();

        // 1) Bindings in the symbol body
        collect_type_alias_bindings(&symbol_dyn, model, &mut bindings, false);

        // 2) Bindings declared in extensions
        let extensions = model.query(ExtensionsFor {
            target_id: self.symbol_id,
        });
        for ext in &extensions {
            let ext_dyn: Arc<dyn Symbol<KestrelLanguage>> = ext.clone();
            collect_type_alias_bindings(&ext_dyn, model, &mut bindings, false);
        }

        // 3) For structs: also collect bindings from protocol extensions
        // (When a struct conforms to a protocol that has extensions adding more
        // conformances, we need type alias bindings from those protocol extensions.)
        if symbol_dyn.metadata().kind() == KestrelSymbolKind::Struct {
            collect_protocol_extension_bindings(&symbol_dyn, model, &mut bindings);
        }

        bindings
    }
}

/// Collect type alias bindings from a symbol's children.
///
/// When `or_insert_only` is true, only inserts if the key doesn't already exist
/// (used for protocol extension bindings where struct bindings take precedence).
fn collect_type_alias_bindings(
    parent: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    bindings: &mut HashMap<String, SignatureType>,
    or_insert_only: bool,
) {
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
        let sig_type = SignatureType::from_ty(&resolved);

        // Check for ConformsToBehavior(s) to get qualified names
        for behavior in type_alias.metadata().behaviors() {
            if let Ok(conforms_to) = behavior.downcast_arc::<ConformsToBehavior>() {
                let qualified_name = format!(
                    "{}.{}",
                    conforms_to.protocol().metadata().name().value,
                    name
                );
                if or_insert_only {
                    bindings.entry(qualified_name).or_insert(sig_type.clone());
                } else {
                    bindings.insert(qualified_name, sig_type.clone());
                }
            }
        }

        // Also insert with simple name
        if or_insert_only {
            bindings.entry(name).or_insert(sig_type);
        } else {
            bindings.insert(name, sig_type);
        }
    }
}

/// Collect type alias bindings from protocol extensions (struct-only).
///
/// When a struct conforms to protocols that have extensions adding more conformances,
/// collect those protocol extension type alias bindings.
fn collect_protocol_extension_bindings(
    symbol_dyn: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &SemanticModel,
    bindings: &mut HashMap<String, SignatureType>,
) {
    let struct_conformances = symbol_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
        .map(|cb| cb.conformances().to_vec())
        .unwrap_or_default();

    // Collect all protocols including inherited ones
    let mut all_protocol_ids = Vec::new();
    let mut to_check = struct_conformances;
    while let Some(conf) = to_check.pop() {
        if let TyKind::Protocol { symbol, .. } = conf.kind() {
            let proto_id = symbol.metadata().id();
            if !all_protocol_ids.contains(&proto_id) {
                all_protocol_ids.push(proto_id);

                if let Some(inherited) = symbol.metadata().get_behavior::<ConformancesBehavior>() {
                    for inherited_conf in inherited.conformances() {
                        to_check.push(inherited_conf.clone());
                    }
                }
            }
        }
    }

    // For each protocol, get its extensions and collect type alias bindings
    for proto_id in all_protocol_ids {
        let proto_extensions = model.query(ExtensionsFor {
            target_id: proto_id,
        });
        for ext in proto_extensions {
            let ext_dyn: Arc<dyn Symbol<KestrelLanguage>> = ext.clone();
            // Use or_insert_only so struct bindings take precedence
            collect_type_alias_bindings(&ext_dyn, model, bindings, true);
        }
    }
}
