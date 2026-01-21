use semantic_tree::behavior::Behavior;
use semantic_tree::symbol::SymbolId;

use crate::behavior::KestrelBehaviorKind;
use crate::behavior::callable::SignatureType;
use crate::language::KestrelLanguage;

/// ImplementsBehavior tracks which protocol method(s) a struct method implements.
///
/// This behavior is attached to struct methods during the BIND phase after:
/// - Protocol conformances are resolved
/// - Method signatures are available
/// - Type substitutions can be performed (Self, associated types, generics)
///
/// A method can implement at most ONE protocol method. If a method signature
/// would satisfy multiple protocol requirements, it's considered ambiguous and
/// an error is reported.
///
/// # Example
/// ```ignore
/// protocol Drawable {
///     func draw()
/// }
///
/// struct Circle: Drawable {
///     func draw() { }  // This method gets ImplementsBehavior(Drawable, draw)
/// }
/// ```
///
/// For generic protocols like `Convertible[T]`, different instantiations create
/// different conformances. The `conformance_signature` field stores the full
/// instantiated protocol type (e.g., "Convertible[Int8]") to distinguish between
/// implementing `Convertible[Int8].init(from:)` vs `Convertible[Int32].init(from:)`.
#[derive(Debug, Clone)]
pub struct ImplementsBehavior {
    /// The protocol that defines the method being implemented
    protocol: SymbolId,

    /// The protocol method being implemented
    protocol_method: SymbolId,

    /// The full conformance signature (e.g., "Convertible[Int8]").
    /// This distinguishes between different instantiations of the same generic protocol.
    conformance_signature: SignatureType,
}

impl Behavior<KestrelLanguage> for ImplementsBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Implements
    }
}

impl ImplementsBehavior {
    /// Create a new ImplementsBehavior linking a struct method to a protocol method
    pub fn new(protocol: SymbolId, protocol_method: SymbolId) -> Self {
        ImplementsBehavior {
            protocol,
            protocol_method,
            conformance_signature: SignatureType::Unit, // Legacy default
        }
    }

    /// Create a new ImplementsBehavior with full conformance signature
    pub fn with_conformance(
        protocol: SymbolId,
        protocol_method: SymbolId,
        conformance_signature: SignatureType,
    ) -> Self {
        ImplementsBehavior {
            protocol,
            protocol_method,
            conformance_signature,
        }
    }

    /// Get the protocol symbol ID
    pub fn protocol(&self) -> SymbolId {
        self.protocol
    }

    /// Get the protocol method symbol ID
    pub fn protocol_method(&self) -> SymbolId {
        self.protocol_method
    }

    /// Get the full conformance signature.
    /// This distinguishes between different instantiations of generic protocols.
    pub fn conformance_signature(&self) -> &SignatureType {
        &self.conformance_signature
    }
}
