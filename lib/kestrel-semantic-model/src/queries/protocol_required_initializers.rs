//! ProtocolRequiredInitializers query - collect required initializers for a protocol (including inherited)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableSignature;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ExtensionsFor, SymbolFor};
use crate::query::Query;

/// Get all initializers required by a protocol, including inherited protocol initializers.
///
/// If a protocol defines an initializer with the same signature as an inherited initializer,
/// the protocol's initializer overrides the inherited initializer.
pub struct ProtocolRequiredInitializers {
    pub protocol_id: SymbolId,
}

impl Query for ProtocolRequiredInitializers {
    type Output = Vec<(CallableSignature, Arc<InitializerSymbol>)>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = match model.query(SymbolFor {
            id: self.protocol_id,
        }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return Vec::new();
        }
        let Ok(protocol) = symbol.downcast_arc::<ProtocolSymbol>() else {
            return Vec::new();
        };

        let mut initializers: HashMap<CallableSignature, Arc<InitializerSymbol>> = HashMap::new();
        let mut visited: HashSet<SymbolId> = HashSet::new();
        collect_protocol_initializers_recursive(&protocol, model, &mut initializers, &mut visited);

        // Collect default implementations from protocol extensions
        let default_implementations = collect_default_initializer_implementations(&protocol, model);

        // Remove initializers that have default implementations
        for sig in default_implementations.keys() {
            initializers.remove(sig);
        }

        initializers.into_iter().collect()
    }
}

#[allow(clippy::only_used_in_recursion)]
fn collect_protocol_initializers_recursive(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    initializers: &mut HashMap<CallableSignature, Arc<InitializerSymbol>>,
    visited: &mut HashSet<SymbolId>,
) {
    let id = protocol.metadata().id();
    if visited.contains(&id) {
        return;
    }
    visited.insert(id);

    let protocol_dyn: Arc<dyn Symbol<KestrelLanguage>> = protocol.clone();
    if let Some(conformances) = protocol_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for inherited_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                collect_protocol_initializers_recursive(symbol, model, initializers, visited);
            }
        }
    }

    for init in collect_initializers_from_symbol(&protocol_dyn) {
        initializers.insert(init.signature(), init);
    }
}

fn collect_initializers_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<InitializerSymbol>> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Initializer)
        .filter_map(|child| child.into_any_arc().downcast::<InitializerSymbol>().ok())
        .collect()
}

/// Collect initializers with default implementations from protocol extensions.
///
/// This function finds all extensions on a protocol (and its inherited protocols)
/// and collects initializers that have bodies (default implementations).
fn collect_default_initializer_implementations(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
) -> HashMap<CallableSignature, Arc<InitializerSymbol>> {
    let mut default_inits: HashMap<CallableSignature, Arc<InitializerSymbol>> = HashMap::new();
    let mut visited: HashSet<SymbolId> = HashSet::new();

    collect_default_initializer_implementations_recursive(
        protocol,
        model,
        &mut default_inits,
        &mut visited,
    );

    default_inits
}

/// Recursively collect default initializer implementations from protocol extensions,
/// including inherited protocols.
fn collect_default_initializer_implementations_recursive(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    default_inits: &mut HashMap<CallableSignature, Arc<InitializerSymbol>>,
    visited: &mut HashSet<SymbolId>,
) {
    let protocol_id = protocol.metadata().id();
    if visited.contains(&protocol_id) {
        return;
    }
    visited.insert(protocol_id);

    // First, collect from inherited protocols
    let protocol_dyn: Arc<dyn Symbol<KestrelLanguage>> = protocol.clone();
    if let Some(conformances) = protocol_dyn
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for inherited_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                collect_default_initializer_implementations_recursive(
                    symbol,
                    model,
                    default_inits,
                    visited,
                );
            }
        }
    }

    // Then collect from extensions on this protocol
    let extensions = model.query(ExtensionsFor {
        target_id: protocol_id,
    });

    for extension in extensions {
        let extension_dyn: Arc<dyn Symbol<KestrelLanguage>> = extension.clone();
        let inits = collect_initializers_from_symbol(&extension_dyn);

        for init in inits {
            // All initializers in protocol extensions are default implementations
            default_inits.insert(init.signature(), init);
        }
    }
}
