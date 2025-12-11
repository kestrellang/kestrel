//! ResolveName query - resolve a name in a given scope context

use std::sync::Arc;

use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::queries::ScopeFor;
use crate::query::Query;
use crate::resolution::SymbolResolution;
use crate::SemanticModel;

/// Resolve a name in a given scope context.
///
/// Walks up the scope chain checking imports, then declarations,
/// then special cases like extension type parameters and inherited
/// protocol members.
pub struct ResolveName {
    pub name: String,
    pub context: SymbolId,
}

impl Query for ResolveName {
    type Output = SymbolResolution;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut current = Some(self.context);

        while let Some(id) = current {
            let scope = model.query(ScopeFor { symbol_id: id });

            // Check imports first
            if let Some(imported) = scope.imports.get(&self.name) {
                return if imported.len() == 1 {
                    SymbolResolution::Found(imported.clone())
                } else {
                    SymbolResolution::Ambiguous(imported.clone())
                };
            }

            // Check declarations
            if let Some(declared) = scope.declarations.get(&self.name) {
                return if declared.len() == 1 {
                    SymbolResolution::Found(declared.clone())
                } else {
                    SymbolResolution::Ambiguous(declared.clone())
                };
            }

            // Check type parameters for extensions
            // Extensions reference type parameters from their target type
            if let Some(symbol) = model.registry().get(id) {
                if symbol.metadata().kind() == KestrelSymbolKind::Extension {
                    if let Some(result) = find_in_extension_type_params(&symbol, &self.name) {
                        return result;
                    }
                }

                // Check inherited associated types from parent protocols
                // (conformances are resolved after scope computation, so we check at lookup time)
                if symbol.metadata().kind() == KestrelSymbolKind::Protocol {
                    if let Some(result) = find_in_inherited_protocols(&symbol, &self.name) {
                        return result;
                    }
                }
            }

            current = scope.parent;
        }

        SymbolResolution::NotFound
    }
}

/// Find a type parameter in an extension's referenced type parameters.
///
/// Extensions reference type parameters from their target struct. When resolving
/// names in extension methods, we need to check these referenced parameters.
fn find_in_extension_type_params(
    extension: &Arc<dyn Symbol<KestrelLanguage>>,
    name: &str,
) -> Option<SymbolResolution> {
    let target_beh = extension.extension_target_behavior()?;

    for type_param in target_beh.referenced_type_parameters() {
        if type_param.metadata().name().value == name {
            return Some(SymbolResolution::Found(vec![type_param.metadata().id()]));
        }
    }

    None
}

/// Search for a name in inherited protocols (for associated type inheritance).
fn find_in_inherited_protocols(
    protocol: &Arc<dyn Symbol<KestrelLanguage>>,
    name: &str,
) -> Option<SymbolResolution> {
    let conformances_beh = protocol.conformances_behavior()?;

    for parent_ty in conformances_beh.conformances() {
        if let TyKind::Protocol {
            symbol: parent_proto,
            ..
        } = parent_ty.kind()
        {
            // Check direct children of parent protocol
            let parent_dyn = parent_proto.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for child in parent_dyn.metadata().children() {
                if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                    && child.metadata().name().value == name
                {
                    return Some(SymbolResolution::Found(vec![child.metadata().id()]));
                }
            }

            // Recursively check grandparent protocols
            if let Some(result) = find_in_inherited_protocols(&parent_dyn, name) {
                return Some(result);
            }
        }
    }

    None
}
