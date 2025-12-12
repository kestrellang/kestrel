//! ProtocolMethodsWithDefiner query - collect protocol methods with their defining protocol

use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::{ConformancesForSymbol, SymbolFor};
use crate::query::Query;

/// Collect all methods visible to a protocol, paired with the protocol that defined each method.
///
/// This includes inherited protocol methods. The returned list may contain multiple entries with
/// the same method signature from different definers; callers can decide how to handle ambiguity.
pub struct ProtocolMethodsWithDefiner {
    pub protocol_id: SymbolId,
}

impl Query for ProtocolMethodsWithDefiner {
    type Output = Vec<(Arc<ProtocolSymbol>, Arc<FunctionSymbol>)>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor {
            id: self.protocol_id,
        }) else {
            return Vec::new();
        };
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return Vec::new();
        }
        let Ok(protocol) = symbol.downcast_arc::<ProtocolSymbol>() else {
            return Vec::new();
        };

        let mut out = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(protocol);

        while let Some(protocol) = queue.pop_front() {
            let id = protocol.metadata().id();
            if visited.contains(&id) {
                continue;
            }
            visited.insert(id);

            for inherited_ty in model.query(ConformancesForSymbol { symbol_id: id }) {
                if let TyKind::Protocol { symbol, .. } = inherited_ty.kind() {
                    queue.push_back(symbol.clone());
                }
            }

            let protocol_dyn: Arc<dyn Symbol<KestrelLanguage>> = protocol.clone();
            for child in protocol_dyn.metadata().children() {
                if child.metadata().kind() != KestrelSymbolKind::Function {
                    continue;
                }
                let Ok(method) = child.into_any_arc().downcast::<FunctionSymbol>() else {
                    continue;
                };
                out.push((protocol.clone(), method));
            }
        }

        out
    }
}
