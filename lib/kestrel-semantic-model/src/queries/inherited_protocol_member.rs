//! InheritedProtocolMember query - search inherited protocols for a member

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Search inherited protocols for a member (e.g., associated type).
///
/// Given a protocol and a name, searches the protocol's parent protocols
/// (via conformances) for a child with the given name.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct InheritedProtocolMember {
    pub protocol_id: SymbolId,
    pub name: String,
}

impl Query for InheritedProtocolMember {
    type Output = Option<SymbolId>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let protocol = model.query(SymbolFor {
            id: self.protocol_id,
        })?;
        let conformances_beh = protocol.metadata().get_behavior::<ConformancesBehavior>()?;

        for parent_ty in conformances_beh.conformances() {
            if let TyKind::Protocol {
                symbol: parent_proto,
                ..
            } = parent_ty.kind()
            {
                // Check direct children of parent protocol
                for child in parent_proto.metadata().children() {
                    if child.metadata().kind() == KestrelSymbolKind::AssociatedType
                        && child.metadata().name().value == self.name
                    {
                        return Some(child.metadata().id());
                    }
                }

                // Recursively check grandparent protocols
                if let Some(result) = model.query(InheritedProtocolMember {
                    protocol_id: parent_proto.metadata().id(),
                    name: self.name.clone(),
                }) {
                    return Some(result);
                }
            }
        }

        None
    }
}
