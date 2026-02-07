//! AssociatedTypeBindingsForStruct query - map type aliases to associated-type bindings for a struct

use std::collections::HashMap;

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
                let sig_type = SignatureType::from_ty(&resolved);

                // Check for ConformsToBehavior(s) to get qualified names.
                // A type alias may satisfy multiple protocols' associated types,
                // so we iterate over all behaviors.
                for behavior in type_alias.metadata().behaviors() {
                    if let Ok(conforms_to) = behavior.downcast_arc::<ConformsToBehavior>() {
                        // Insert with qualified key: "Protocol.Name"
                        let qualified_name = format!(
                            "{}.{}",
                            conforms_to.protocol().metadata().name().value,
                            name
                        );
                        bindings.insert(qualified_name, sig_type.clone());
                    }
                }

                // Also insert with simple name for backward compatibility
                // (handles unqualified lookups and module-level type aliases)
                bindings.insert(name, sig_type);
            }
        };

        // 1) Bindings in the struct body
        collect_from_symbol(&struct_dyn);

        // 2) Bindings declared in extensions of this struct
        let extensions = model.query(ExtensionsFor {
            target_id: self.struct_id,
        });
        for ext in &extensions {
            let ext_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = ext.clone();
            collect_from_symbol(&ext_dyn);
        }

        // 3) Bindings from protocol extensions
        // When a struct conforms to a protocol that has extensions adding more conformances,
        // we need to collect the type alias bindings from those protocol extensions.
        // e.g., if Int64: Equatable and `extend Equatable: Equal { type Equal.Output = Bool }`,
        // then Int64 gets the binding Equal.Output = Bool.
        let struct_conformances = struct_dyn
            .metadata()
            .get_behavior::<ConformancesBehavior>()
            .map(|cb| cb.conformances().to_vec())
            .unwrap_or_default();

        // Collect all protocols including inherited ones
        let mut all_protocol_ids = Vec::new();
        let mut to_check = struct_conformances.clone();
        while let Some(conf) = to_check.pop() {
            if let TyKind::Protocol { symbol, .. } = conf.kind() {
                let proto_id = symbol.metadata().id();
                if !all_protocol_ids.contains(&proto_id) {
                    all_protocol_ids.push(proto_id);

                    // Add inherited protocols
                    if let Some(inherited) =
                        symbol.metadata().get_behavior::<ConformancesBehavior>()
                    {
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
                let ext_dyn: std::sync::Arc<dyn Symbol<KestrelLanguage>> = ext.clone();
                // Collect type aliases from this protocol extension
                for child in ext_dyn.metadata().children() {
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

                    // Use ConformsToBehavior to get qualified names
                    for behavior in type_alias.metadata().behaviors() {
                        if let Ok(conforms_to) = behavior.downcast_arc::<ConformsToBehavior>() {
                            let qualified_name = format!(
                                "{}.{}",
                                conforms_to.protocol().metadata().name().value,
                                name
                            );
                            // Only insert if not already defined by the struct
                            // (struct bindings take precedence)
                            bindings.entry(qualified_name).or_insert(sig_type.clone());
                        }
                    }

                    // Also insert with simple name (but don't override struct bindings)
                    bindings.entry(name).or_insert(sig_type);
                }
            }
        }

        bindings
    }
}
