//! Solution types for type inference results.
//!
//! After solving constraints, the inference context produces a [`Solution`]
//! containing resolved types, value resolutions, and any errors encountered.

use std::collections::HashMap;

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyId};
use semantic_tree::symbol::SymbolId;

use crate::error::InferenceError;

/// Describes what kind of member was resolved for a deferred member access.
///
/// Used by the apply phase to produce the correct concrete ExprKind.
#[derive(Debug, Clone)]
pub enum MemberKind {
    /// Struct field — produce FieldAccess
    Field { mutable: bool },
    /// Computed property — produce a getter call (SymbolRef + Call)
    ComputedProperty { has_setter: bool },
    /// Protocol property — produce ProtocolPropertyAccess
    ProtocolProperty {
        protocol_id: SymbolId,
        has_setter: bool,
        is_static: bool,
    },
    /// Method reference (not called) — produce MethodRef
    Method,
}

/// A resolved value from type-directed member access.
///
/// When a member access depends on type inference (e.g., `x.method()` where
/// `x`'s type is inferred), the solution tracks which symbol was ultimately
/// resolved.
#[derive(Debug, Clone)]
pub struct ValueResolution {
    /// The symbol that was resolved
    pub symbol_id: SymbolId,
    /// Type argument substitutions for the member
    pub substitutions: Substitutions,
}

impl ValueResolution {
    /// Create a new value resolution.
    pub fn new(symbol_id: SymbolId, substitutions: Substitutions) -> Self {
        Self {
            symbol_id,
            substitutions,
        }
    }

    /// Create a value resolution with no substitutions.
    pub fn simple(symbol_id: SymbolId) -> Self {
        Self {
            symbol_id,
            substitutions: Substitutions::new(),
        }
    }
}

/// Information about a value promotion.
///
/// When an expression needs to be wrapped with `FromValue.from()`,
/// this records the target type and resolved method for the transformation.
#[derive(Debug, Clone)]
pub struct PromotionInfo {
    /// The target type (e.g., `Optional[Int]`)
    pub target_ty: Ty,
    /// The resolved `FromValue.from` method symbol
    pub from_method: SymbolId,
    /// Type substitutions for the method call
    pub substitutions: Substitutions,
}

impl PromotionInfo {
    /// Create a new promotion info.
    pub fn new(target_ty: Ty, from_method: SymbolId, substitutions: Substitutions) -> Self {
        Self {
            target_ty,
            from_method,
            substitutions,
        }
    }
}

/// The solution to a set of type inference constraints.
///
/// Contains:
/// - Resolved types for all inference placeholders
/// - Resolved symbols for type-directed member accesses
/// - Promotions for expressions that need wrapping
/// - Any errors encountered during inference
#[derive(Debug, Clone, Default)]
pub struct Solution {
    /// Resolved types indexed by their TyId.
    ///
    /// Each inference placeholder (`TyKind::Infer`) gets an entry here
    /// with its resolved concrete type.
    pub types: HashMap<TyId, Ty>,

    /// Resolved values indexed by their ExprId.
    ///
    /// Member accesses that depended on type inference get entries here
    /// with their resolved symbol and substitutions.
    pub values: HashMap<ExprId, ValueResolution>,

    /// Promotions indexed by their ExprId.
    ///
    /// Expressions that need to be wrapped with `FromValue.from()` get entries
    /// here with information about the target type and method to call.
    pub promotions: HashMap<ExprId, PromotionInfo>,

    /// Member kinds for deferred member access expressions.
    ///
    /// Records what kind of member was resolved (field, computed property,
    /// protocol property, method) so the apply phase can produce the correct ExprKind.
    pub member_kinds: HashMap<ExprId, MemberKind>,

    /// Errors encountered during type inference.
    ///
    /// The solver accumulates errors rather than failing fast, allowing
    /// multiple type errors to be reported in a single pass.
    pub errors: Vec<InferenceError>,
}

impl Solution {
    /// Create an empty solution.
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            values: HashMap::new(),
            promotions: HashMap::new(),
            member_kinds: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Create a solution with the given type and value mappings.
    pub fn with_mappings(
        types: HashMap<TyId, Ty>,
        values: HashMap<ExprId, ValueResolution>,
    ) -> Self {
        Self {
            types,
            values,
            promotions: HashMap::new(),
            member_kinds: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Create a solution with the given type mappings, value mappings, and errors.
    pub fn with_errors(
        types: HashMap<TyId, Ty>,
        values: HashMap<ExprId, ValueResolution>,
        errors: Vec<InferenceError>,
    ) -> Self {
        Self {
            types,
            values,
            promotions: HashMap::new(),
            member_kinds: HashMap::new(),
            errors,
        }
    }

    /// Create a solution with all mappings including promotions.
    pub fn with_promotions(
        types: HashMap<TyId, Ty>,
        values: HashMap<ExprId, ValueResolution>,
        promotions: HashMap<ExprId, PromotionInfo>,
        errors: Vec<InferenceError>,
    ) -> Self {
        Self {
            types,
            values,
            promotions,
            member_kinds: HashMap::new(),
            errors,
        }
    }

    /// Get the resolved type for a TyId.
    pub fn get_type(&self, id: TyId) -> Option<&Ty> {
        self.types.get(&id)
    }

    /// Get the resolved value for an ExprId.
    pub fn get_value(&self, id: ExprId) -> Option<&ValueResolution> {
        self.values.get(&id)
    }

    /// Get the promotion info for an ExprId.
    pub fn get_promotion(&self, id: ExprId) -> Option<&PromotionInfo> {
        self.promotions.get(&id)
    }

    /// Get the member kind for a deferred member access expression.
    pub fn get_member_kind(&self, id: ExprId) -> Option<&MemberKind> {
        self.member_kinds.get(&id)
    }

    /// Set the member kind for a deferred member access expression.
    pub fn set_member_kind(&mut self, id: ExprId, kind: MemberKind) {
        self.member_kinds.insert(id, kind);
    }

    /// Check if a type has been resolved.
    pub fn has_type(&self, id: TyId) -> bool {
        self.types.contains_key(&id)
    }

    /// Check if a value has been resolved.
    pub fn has_value(&self, id: ExprId) -> bool {
        self.values.contains_key(&id)
    }

    /// Get the number of resolved types.
    pub fn type_count(&self) -> usize {
        self.types.len()
    }

    /// Get the number of resolved values.
    pub fn value_count(&self) -> usize {
        self.values.len()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the errors from inference.
    pub fn errors(&self) -> &[InferenceError] {
        &self.errors
    }

    /// Get mutable access to errors.
    pub fn errors_mut(&mut self) -> &mut Vec<InferenceError> {
        &mut self.errors
    }

    /// Add an error to the solution.
    pub fn add_error(&mut self, error: InferenceError) {
        self.errors.push(error);
    }
}
