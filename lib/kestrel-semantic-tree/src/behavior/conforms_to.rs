use std::sync::Arc;

use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::SymbolId;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;
use crate::symbol::protocol::ProtocolSymbol;

/// Behavior for type alias symbols that bind associated types.
///
/// When a struct conforms to a protocol and provides a binding for an
/// associated type (e.g., `type Item = Int`), the resulting TypeAliasSymbol
/// has this behavior to indicate which protocol's associated type it satisfies.
///
/// # Example
///
/// ```kestrel
/// protocol Iterator {
///     type Item;  // AssociatedTypeSymbol
/// }
///
/// struct IntRange: Iterator {
///     type Item = Int;  // TypeAliasSymbol with ConformsToBehavior
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConformsToBehavior {
    /// The protocol whose associated type this binding satisfies
    protocol: Arc<ProtocolSymbol>,
    /// The name of the associated type being bound
    associated_type_name: String,
    /// The symbol ID of the associated type in the protocol
    associated_type_id: Option<SymbolId>,
}

impl Behavior<KestrelLanguage> for ConformsToBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::ConformsTo
    }
}

impl ConformsToBehavior {
    /// Create a new ConformsToBehavior
    pub fn new(
        protocol: Arc<ProtocolSymbol>,
        associated_type_name: String,
        associated_type_id: Option<SymbolId>,
    ) -> Self {
        ConformsToBehavior {
            protocol,
            associated_type_name,
            associated_type_id,
        }
    }

    /// Get the protocol this binding is for
    pub fn protocol(&self) -> &Arc<ProtocolSymbol> {
        &self.protocol
    }

    /// Get the name of the associated type being bound
    pub fn associated_type_name(&self) -> &str {
        &self.associated_type_name
    }

    /// Get the symbol ID of the associated type, if resolved
    pub fn associated_type_id(&self) -> Option<SymbolId> {
        self.associated_type_id
    }
}
