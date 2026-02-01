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
    /// Parameter types for method calls (empty for field access).
    /// Used to create constraints between argument types and parameter types
    /// during type inference, enabling proper type inference for literals.
    pub parameters: Vec<Ty>,
    /// True if the method's return type was `Self` before substitution.
    /// Used to enable bidirectional type inference: when a method returns Self,
    /// the expected result type can propagate back to constrain the receiver type.
    /// This enables patterns like `-32768` to infer as Int16 when context expects Int16.
    pub returns_self: bool,
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

    /// Get the protocol that a method belongs to, if any.
    ///
    /// Used to generate conformance constraints when calling protocol methods.
    /// Returns None if the method is not a builtin protocol method.
    fn protocol_for_method(&self, method_id: SymbolId) -> Option<SymbolId> {
        // Default implementation returns None.
        // Implementations with access to the builtin registry can check if the method
        // is a builtin protocol method and return the associated protocol.
        let _ = method_id;
        None
    }

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

    /// Get the default type for string literals when type is ambiguous.
    ///
    /// Returns the `String` struct type if available in the stdlib,
    /// otherwise falls back to the primitive string type.
    fn default_string_type(&self, span: Span) -> Ty {
        // Default implementation uses primitive string type.
        // Implementations with access to stdlib can override to return the String struct.
        Ty::string(span)
    }

    /// Get the default type for boolean literals when type is ambiguous.
    ///
    /// Returns the `Bool` struct type if available in the stdlib,
    /// otherwise falls back to the primitive bool type.
    fn default_boolean_type(&self, span: Span) -> Ty {
        // Default implementation uses primitive bool type.
        // Implementations with access to stdlib can override to return the Bool struct.
        Ty::bool(span)
    }

    /// Get the default type for character literals when type is ambiguous.
    ///
    /// Returns the `CodePoint` struct type if available in the stdlib,
    /// otherwise falls back to i32.
    fn default_char_type(&self, span: Span) -> Ty {
        // Default implementation uses i32.
        // Implementations with access to stdlib can override to return the CodePoint struct.
        Ty::int(IntBits::I32, span)
    }

    /// Get the default type for array literals when type is ambiguous.
    ///
    /// Creates `Array[element_ty]` struct type. Returns None if the Array struct
    /// is not available (e.g., stdlib not loaded).
    fn default_array_type(&self, element_ty: Ty, span: Span) -> Option<Ty>;

    /// Check if target_ty conforms to FromValue[source_ty].
    ///
    /// Used by the Promotable constraint to determine if a value can be
    /// implicitly wrapped. Returns the from() method symbol and substitutions
    /// if the conformance exists.
    ///
    /// # Arguments
    ///
    /// * `target_ty` - The target type (e.g., `Optional[Int]`)
    /// * `source_ty` - The source type (e.g., `Int`)
    ///
    /// # Returns
    ///
    /// If target_ty conforms to FromValue[source_ty], returns (method_id, substitutions).
    /// Otherwise returns None.
    fn check_from_value_conformance(
        &self,
        target_ty: &Ty,
        source_ty: &Ty,
    ) -> Option<(SymbolId, Substitutions)> {
        // Default implementation returns None.
        // Implementors with access to conformance checking can override.
        let _ = (target_ty, source_ty);
        None
    }
}
