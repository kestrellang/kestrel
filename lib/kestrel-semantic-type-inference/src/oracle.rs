//! Type oracle trait for decoupled type resolution.
//!
//! The [`TypeOracle`] trait allows the inference solver to query type information
//! without depending directly on the semantic model implementation.

use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Substitutions, Ty};
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

/// Describes a successfully resolved member access.
#[derive(Debug, Clone)]
pub struct MemberResolution {
    /// The type of the member (field type, method return type, etc.)
    pub ty: Ty,
    /// The symbol ID of the resolved member
    pub symbol_id: SymbolId,
    /// Type argument substitutions from the receiver type
    pub substitutions: Substitutions,
}

/// Error when member resolution fails.
#[derive(Debug, Clone)]
pub enum MemberError {
    /// The member was not found on the type
    NotFound {
        /// The type that was searched
        receiver_ty: Ty,
        /// The member name that wasn't found
        member: String,
    },
    /// The type is not yet known (inference placeholder)
    UnknownType,
    /// Multiple ambiguous candidates found
    Ambiguous {
        /// Number of candidates
        count: usize,
    },
}

/// Trait for querying type information during inference.
///
/// This trait decouples the inference solver from the semantic model,
/// allowing the solver to be tested independently and potentially
/// reused with different backends.
///
/// Implementors should cache lookups where possible for performance.
pub trait TypeOracle {
    /// Look up a member on a type.
    ///
    /// # Arguments
    ///
    /// * `receiver_ty` - The type being accessed
    /// * `member` - The member name to look up
    /// * `is_static` - True for static member access (Type.member), false for instance access
    ///
    /// # Returns
    ///
    /// * `Ok(resolution)` - The member was found with its type and symbol
    /// * `Err(MemberError)` - The lookup failed for the given reason
    fn resolve_member(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
    ) -> Result<MemberResolution, MemberError>;

    /// Check if a type conforms to a protocol.
    ///
    /// # Arguments
    ///
    /// * `ty` - The type to check
    /// * `protocol_id` - The protocol symbol ID
    ///
    /// # Returns
    ///
    /// True if the type conforms to the protocol, false otherwise.
    fn conforms_to(&self, ty: &Ty, protocol_id: SymbolId) -> bool;

    /// Resolve an associated type on a container type.
    ///
    /// # Arguments
    ///
    /// * `container` - The type containing the associated type (e.g., a protocol or struct implementing a protocol)
    /// * `assoc_name` - The name of the associated type
    ///
    /// # Returns
    ///
    /// The resolved associated type, or None if not found.
    fn resolve_associated_type(&self, container: &Ty, assoc_name: &str) -> Option<Ty>;

    /// Get the underlying type for a type alias.
    ///
    /// Used to expand type aliases when comparing types during unification.
    ///
    /// # Returns
    ///
    /// The expanded type, or the original if not a type alias.
    fn expand_type_alias(&self, ty: &Ty) -> Ty {
        // Default implementation uses Ty::expand_aliases
        ty.expand_aliases()
    }

    /// Get the name of a symbol by its ID.
    ///
    /// Used for error messages when reporting conformance failures
    /// or other diagnostics that need to display symbol names.
    ///
    /// # Arguments
    ///
    /// * `symbol_id` - The symbol ID to look up
    ///
    /// # Returns
    ///
    /// The symbol's name, or None if not found.
    fn symbol_name(&self, symbol_id: SymbolId) -> Option<String>;

    /// Get the symbol ID for a builtin protocol.
    ///
    /// Used by type inference to look up ExpressibleBy* protocols for literal type inference.
    ///
    /// # Arguments
    ///
    /// * `feature` - The language feature to look up (e.g., ExpressibleByIntLiteral)
    ///
    /// # Returns
    ///
    /// The protocol's symbol ID, or None if not registered.
    fn builtin_protocol(&self, feature: LanguageFeature) -> Option<SymbolId>;

    /// Get the default type for integer literals when type is ambiguous.
    ///
    /// Returns Int64 by default.
    fn default_integer_type(&self, span: Span) -> Ty {
        Ty::int(IntBits::I64, span)
    }

    /// Get the default type for float literals when type is ambiguous.
    ///
    /// Returns Float64 by default.
    fn default_float_type(&self, span: Span) -> Ty {
        Ty::float(FloatBits::F64, span)
    }
}
