//! Solution types for type inference results.
//!
//! After solving constraints, the inference context produces a [`Solution`]
//! containing resolved types and value resolutions.

use std::collections::HashMap;

use kestrel_semantic_tree::expr::ExprId;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyId};
use semantic_tree::symbol::SymbolId;

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

/// The solution to a set of type inference constraints.
///
/// Contains:
/// - Resolved types for all inference placeholders
/// - Resolved symbols for type-directed member accesses
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
}

impl Solution {
    /// Create an empty solution.
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            values: HashMap::new(),
        }
    }

    /// Create a solution with the given type and value mappings.
    pub fn with_mappings(
        types: HashMap<TyId, Ty>,
        values: HashMap<ExprId, ValueResolution>,
    ) -> Self {
        Self { types, values }
    }

    /// Get the resolved type for a TyId.
    pub fn get_type(&self, id: TyId) -> Option<&Ty> {
        self.types.get(&id)
    }

    /// Get the resolved value for an ExprId.
    pub fn get_value(&self, id: ExprId) -> Option<&ValueResolution> {
        self.values.get(&id)
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
}
