//! ProtocolRequiredMethods query - collect required methods for a protocol (including inherited)

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableSignature;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get all methods required by a protocol, including inherited protocol methods.
///
/// If a protocol defines a method with the same signature as an inherited method,
/// the protocol's method overrides the inherited method.
pub struct ProtocolRequiredMethods {
    pub protocol_id: SymbolId,
}

impl Query for ProtocolRequiredMethods {
    type Output = Vec<(CallableSignature, Arc<FunctionSymbol>)>;

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

        let mut methods: HashMap<CallableSignature, Arc<FunctionSymbol>> = HashMap::new();
        let mut visited: HashSet<SymbolId> = HashSet::new();
        collect_protocol_methods_recursive(&protocol, model, &mut methods, &mut visited);
        methods.into_iter().collect()
    }
}

fn collect_protocol_methods_recursive(
    protocol: &Arc<ProtocolSymbol>,
    model: &SemanticModel,
    methods: &mut HashMap<CallableSignature, Arc<FunctionSymbol>>,
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
                collect_protocol_methods_recursive(symbol, model, methods, visited);
            }
        }
    }

    for method in collect_methods_from_symbol(&protocol_dyn) {
        methods.insert(method.signature(), method);
    }
}

fn collect_methods_from_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Vec<Arc<FunctionSymbol>> {
    symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|child| child.metadata().kind() == KestrelSymbolKind::Function)
        .filter_map(|child| child.into_any_arc().downcast::<FunctionSymbol>().ok())
        .collect()
}
