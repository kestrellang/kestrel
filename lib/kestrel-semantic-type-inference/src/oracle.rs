//! Type oracle trait for decoupled type resolution.
//!
//! The [`TypeOracle`] trait allows the inference solver to query type information
//! without depending directly on the semantic model implementation.

use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::ty::{FloatBits, IntBits, Substitutions, Ty, WhereClause};
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::solution::MemberKind;

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
    /// Where clause constraints from the method declaration.
    /// These are converted into inference constraints to enable type parameter inference
    /// from where clause equality constraints like `where Item = Optional[T]`.
    pub where_constraints: WhereClause,
    /// Number of required parameters (those without default values).
    /// Used for arity checking that accounts for default parameters.
    pub required_parameter_count: usize,
    /// The protocol this member came from, if resolved via protocol bounds.
    /// Used by the binder to construct `ProtocolPropertyAccess` expressions
    /// that reference the witness table for protocol dispatch.
    pub protocol_id: Option<SymbolId>,
    /// Whether the property has a setter, if this is a protocol property.
    /// Used by the binder to determine mutability of `ProtocolPropertyAccess`.
    pub has_setter: Option<bool>,
    /// Type parameter SymbolIds for method-level generics (e.g., `func map[T](...)`).
    /// Empty for non-generic methods, fields, and properties.
    /// Used by the solver to zip with explicit type args from the call site.
    pub method_type_param_ids: Vec<SymbolId>,
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
    /// Candidates exist but none match the provided labels/arity
    NoMatchingOverload {
        /// The function/method name
        name: String,
        /// The provided argument labels
        provided_labels: Vec<Option<String>>,
        /// The expected labels from the best candidate
        expected_labels: Vec<Option<String>>,
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

    /// Look up a member on a type with expected argument arity for callable members.
    ///
    /// Default implementation delegates to `resolve_member` and then verifies callable arity.
    fn resolve_member_with_arity(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
        argument_count: usize,
    ) -> Result<MemberResolution, MemberError> {
        let resolution = self.resolve_member(receiver_ty, member, is_static)?;
        if argument_count >= resolution.required_parameter_count
            && argument_count <= resolution.parameters.len()
        {
            Ok(resolution)
        } else {
            Err(MemberError::NotFound {
                receiver_ty: receiver_ty.clone(),
                member: member.to_string(),
            })
        }
    }

    /// Look up a member on a type with expected argument labels for overload resolution.
    ///
    /// Labels include arity information (the length of the labels slice is the argument count).
    /// Each entry is None for unlabeled arguments, Some(label) for labeled ones.
    ///
    /// Default implementation delegates to `resolve_member_with_arity`.
    fn resolve_member_with_labels(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
        labels: &[Option<String>],
    ) -> Result<MemberResolution, MemberError> {
        self.resolve_member_with_arity(receiver_ty, member, is_static, labels.len())
    }

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

    /// Normalize a type using equality constraints from the current context.
    ///
    /// This resolves associated types like `I.Item` to their constrained values
    /// when there's an equality constraint like `I.Item = (K, V)` in scope.
    ///
    /// Used by tuple indexing and other operations that need to know the concrete
    /// type behind an associated type.
    ///
    /// # Arguments
    ///
    /// * `ty` - The type to normalize
    ///
    /// # Returns
    ///
    /// The normalized type, or the original if no normalization applies.
    fn normalize_with_constraints(&self, ty: &Ty) -> Ty {
        // Default implementation returns the type unchanged.
        // Implementors with access to where clause constraints can override.
        ty.clone()
    }

    /// Get the context symbol ID (the function being analyzed), if available.
    ///
    /// Used for visibility checking and context-aware resolution.
    /// Returns None when no function context is available.
    fn context_symbol_id(&self) -> Option<SymbolId> {
        None
    }

    /// Look up a member with full type-directed overload resolution.
    ///
    /// When multiple overloads match by name, arity, and labels, uses argument types
    /// to score each candidate and pick the best match.
    ///
    /// Default implementation delegates to `resolve_member_with_labels`.
    fn resolve_member_full(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
        labels: &[Option<String>],
        argument_types: &[Option<Ty>],
    ) -> Result<MemberResolution, MemberError> {
        let _ = argument_types; // default: ignore argument types
        self.resolve_member_with_labels(receiver_ty, member, is_static, labels)
    }

    /// Look up ALL matching members on a type (for overload candidate collection).
    ///
    /// Unlike `resolve_member` which returns the single best match, this returns
    /// all matching members with the given name. Used by the binder to collect
    /// overload candidates for label-aware disambiguation during type inference.
    ///
    /// Default implementation delegates to `resolve_member` and wraps in a vec.
    fn resolve_all_members(
        &self,
        receiver_ty: &Ty,
        member: &str,
        is_static: bool,
    ) -> Result<Vec<MemberResolution>, MemberError> {
        self.resolve_member(receiver_ty, member, is_static)
            .map(|r| vec![r])
    }

    /// Resolve a subscript call on a type.
    ///
    /// Used for deferred subscript calls where resolution needs full type information.
    /// Finds matching subscript declarations on the receiver type and its protocol conformances.
    ///
    /// # Arguments
    ///
    /// * `receiver_ty` - The type being subscripted
    /// * `labels` - Argument labels (None for unlabeled args)
    /// * `argument_types` - Resolved argument types (None if not yet known)
    ///
    /// # Returns
    ///
    /// * `Ok(resolution)` - The matching subscript with its getter symbol, return type, etc.
    /// * `Err(MemberError)` - No matching subscript found
    fn resolve_subscript(
        &self,
        receiver_ty: &Ty,
        labels: &[Option<String>],
        argument_types: &[Option<Ty>],
    ) -> Result<MemberResolution, MemberError> {
        let _ = (receiver_ty, labels, argument_types);
        Err(MemberError::NotFound {
            receiver_ty: receiver_ty.clone(),
            member: "subscript".to_string(),
        })
    }

    /// Resolve a direct function call.
    ///
    /// Used for deferred function calls where overload resolution needs full type information.
    /// Checks candidates against labels and argument types.
    ///
    /// # Arguments
    ///
    /// * `candidates` - Candidate function symbol IDs
    /// * `labels` - Argument labels (None for unlabeled args)
    /// * `argument_types` - Resolved argument types (None if not yet known)
    ///
    /// # Returns
    ///
    /// * `Ok(resolution)` - The matching function with its symbol, return type, parameters, etc.
    /// * `Err(MemberError)` - No matching function found
    fn resolve_function(
        &self,
        candidates: &[SymbolId],
        labels: &[Option<String>],
        argument_types: &[Option<Ty>],
    ) -> Result<MemberResolution, MemberError> {
        let _ = (candidates, labels, argument_types);
        Err(MemberError::NotFound {
            receiver_ty: Ty::unit(Span::synthetic(0)),
            member: "function".to_string(),
        })
    }

    /// Check if a target symbol is visible from the given context.
    ///
    /// Used as a post-resolution check to filter out private/internal members
    /// that were resolved but shouldn't be accessible from the current context.
    ///
    /// Returns true by default (no visibility checking without a model).
    fn is_visible(&self, _target: SymbolId, _from_context: SymbolId) -> bool {
        true
    }

    /// Resolve an initializer on a struct type.
    ///
    /// Used for deferred init calls where overload resolution needs full type information.
    /// Collects initializers from the struct and its extensions, filters by labels/arity,
    /// and uses argument types for type-directed overload selection.
    ///
    /// # Arguments
    ///
    /// * `struct_ty` - The struct type being initialized
    /// * `labels` - Argument labels (None for unlabeled args)
    /// * `argument_types` - Resolved argument types (None if not yet known)
    ///
    /// # Returns
    ///
    /// * `Ok(resolution)` - The matching initializer with its type and symbol
    /// * `Err(MemberError)` - No matching initializer found
    fn resolve_init(
        &self,
        struct_ty: &Ty,
        labels: &[Option<String>],
        argument_types: &[Option<Ty>],
    ) -> Result<MemberResolution, MemberError> {
        let _ = (struct_ty, labels, argument_types);
        Err(MemberError::NotFound {
            receiver_ty: struct_ty.clone(),
            member: "init".to_string(),
        })
    }

    /// Get the where clause for a function symbol.
    ///
    /// Used to extract where clause constraints from method calls that were
    /// resolved during binding. This is critical for methods like
    /// `compactMap[T]() where Item = Optional[T]` where the where clause
    /// equality constraint is the only way to infer `T`.
    ///
    /// # Arguments
    ///
    /// * `function_id` - The symbol ID of the function
    ///
    /// # Returns
    ///
    /// The function's where clause, or an empty where clause if not found.
    fn function_where_clause(&self, function_id: SymbolId) -> WhereClause {
        // Default implementation returns empty where clause.
        // Implementors with access to the semantic model can override.
        let _ = function_id;
        WhereClause::new()
    }

    /// Classify what kind of member a resolved symbol represents.
    ///
    /// Used by the solver to store member kind metadata for the apply phase
    /// when resolving non-call member access (DeferredMemberAccess).
    ///
    /// # Arguments
    ///
    /// * `receiver_ty` - The receiver type (used for protocol property detection)
    /// * `resolution` - The member resolution from resolve_member
    ///
    /// # Returns
    ///
    /// The kind of member (field, computed property, protocol property, or method).
    fn classify_member(&self, receiver_ty: &Ty, resolution: &MemberResolution) -> MemberKind {
        // Default: if protocol_id is set, it's a protocol property; otherwise method.
        let _ = receiver_ty;
        if let Some(protocol_id) = resolution.protocol_id {
            MemberKind::ProtocolProperty {
                protocol_id,
                has_setter: resolution.has_setter.unwrap_or(false),
                is_static: false,
            }
        } else {
            MemberKind::Method
        }
    }
}
