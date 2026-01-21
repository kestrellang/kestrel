//! ResolveName query - resolve a name in a given scope context

use std::sync::Arc;

use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::FlattenedProtocolBehavior;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{InheritedProtocolMember, ScopeFor, SymbolFor};
use crate::query::Query;
use crate::resolution::SymbolResolution;

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

            // Check whole-module imports (wildcard imports)
            let mut wildcard_candidates = Vec::new();
            let imports = model.query(crate::queries::ImportsInScope { symbol_id: id });
            for import in imports {
                // Only consider whole-module imports (no items, no alias)
                if import.items.is_empty()
                    && import.alias.is_none()
                    && let Ok(module_id) = model.query(crate::queries::ResolveModulePath {
                        path: import.module_path.clone(),
                        context: id,
                    })
                {
                    // Check if the name exists in the module's visible children
                    if let Some(child) = model
                        .query(crate::queries::VisibleChildrenByName {
                            parent: module_id,
                            name: self.name.clone(),
                            context: self.context,
                        })
                        .into_iter()
                        .next()
                    {
                        wildcard_candidates.push(child.metadata().id());
                    }
                }
            }

            if !wildcard_candidates.is_empty() {
                return if wildcard_candidates.len() == 1 {
                    SymbolResolution::Found(wildcard_candidates)
                } else {
                    SymbolResolution::Ambiguous(wildcard_candidates)
                };
            }

            // Check type parameters and associated types for extensions
            // Extensions reference type parameters from their target type
            // Protocol extensions also access associated types from target protocol
            if let Some(symbol) = model.query(SymbolFor { id }) {
                if symbol.metadata().kind() == KestrelSymbolKind::Extension {
                    if let Some(result) = find_in_extension_type_params(&symbol, &self.name) {
                        return result;
                    }
                    if let Some(result) =
                        find_in_protocol_extension_associated_types(&symbol, &self.name)
                    {
                        return result;
                    }
                }

                // Check inherited associated types from parent protocols
                // (conformances are resolved after scope computation, so we check at lookup time)
                if symbol.metadata().kind() == KestrelSymbolKind::Protocol
                    && let Some(member_id) = model.query(InheritedProtocolMember {
                        protocol_id: id,
                        name: self.name.clone(),
                    })
                {
                    return SymbolResolution::Found(vec![member_id]);
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
    let target_beh = extension
        .metadata()
        .get_behavior::<ExtensionTargetBehavior>()?;

    for type_param in target_beh.referenced_type_parameters() {
        if type_param.metadata().name().value == name {
            return Some(SymbolResolution::Found(vec![type_param.metadata().id()]));
        }
    }

    None
}

/// Find an associated type in a protocol extension's target protocol.
///
/// Protocol extensions can access associated types from the target protocol
/// (including inherited ones) in method signatures and bodies.
fn find_in_protocol_extension_associated_types(
    extension: &Arc<dyn Symbol<KestrelLanguage>>,
    name: &str,
) -> Option<SymbolResolution> {
    let target_beh = extension
        .metadata()
        .get_behavior::<ExtensionTargetBehavior>()?;

    let protocol_sym = target_beh.target_protocol()?;

    let flattened = protocol_sym
        .metadata()
        .get_behavior::<FlattenedProtocolBehavior>()?;

    let assoc_type = flattened.associated_types().get(name)?;

    Some(SymbolResolution::Found(vec![
        assoc_type.symbol.metadata().id(),
    ]))
}
