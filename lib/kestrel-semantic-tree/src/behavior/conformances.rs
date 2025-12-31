use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::Symbol;

use crate::{behavior::KestrelBehaviorKind, language::KestrelLanguage, ty::Ty};

/// ConformancesBehavior represents the resolved protocols that a type conforms to,
/// as well as protocols it explicitly does NOT conform to (negative conformances).
///
/// This is used for:
/// - Structs that conform to protocols (e.g., `struct Point: Drawable { }`)
/// - Protocols that inherit from other protocols (e.g., `protocol Shape: Drawable { }`)
/// - Types that opt-out of implicit conformances (e.g., `struct Handle: not Copyable { }`)
///
/// This behavior is added during the bind phase with resolved protocol types.
/// Use the last ConformancesBehavior to get the fully resolved conformances.
#[derive(Debug, Clone)]
pub struct ConformancesBehavior {
    /// The resolved protocol types this symbol conforms to (positive conformances)
    conformances: Vec<Ty>,
    /// The resolved protocol types this symbol explicitly does NOT conform to (negative conformances)
    /// Only valid for builtin protocols that allow negation (e.g., Copyable)
    negative_conformances: Vec<Ty>,
}

impl Behavior<KestrelLanguage> for ConformancesBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Conformances
    }
}

impl ConformancesBehavior {
    /// Create a new ConformancesBehavior with the given resolved conformances
    pub fn new(conformances: Vec<Ty>) -> Self {
        ConformancesBehavior {
            conformances,
            negative_conformances: Vec::new(),
        }
    }

    /// Create a new ConformancesBehavior with both positive and negative conformances
    pub fn with_negatives(conformances: Vec<Ty>, negative_conformances: Vec<Ty>) -> Self {
        ConformancesBehavior {
            conformances,
            negative_conformances,
        }
    }

    /// Get the resolved conformances (protocols this type conforms to)
    pub fn conformances(&self) -> &[Ty] {
        &self.conformances
    }

    /// Get the negative conformances (protocols this type explicitly does NOT conform to)
    pub fn negative_conformances(&self) -> &[Ty] {
        &self.negative_conformances
    }

    /// Check if there are any positive conformances
    pub fn has_conformances(&self) -> bool {
        !self.conformances.is_empty()
    }

    /// Check if there are any negative conformances
    pub fn has_negative_conformances(&self) -> bool {
        !self.negative_conformances.is_empty()
    }

    /// Check if this type has explicitly opted out of a specific protocol
    pub fn has_negative_conformance_to(&self, protocol_id: semantic_tree::symbol::SymbolId) -> bool {
        use crate::ty::TyKind;
        self.negative_conformances.iter().any(|ty| {
            if let TyKind::Protocol { symbol, .. } = ty.kind() {
                symbol.metadata().id() == protocol_id
            } else {
                false
            }
        })
    }
}
