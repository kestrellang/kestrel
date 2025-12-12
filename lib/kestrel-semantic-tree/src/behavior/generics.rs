//! Generics behavior for symbols with type parameters and where clauses.
//!
//! This behavior is attached to functions, structs, protocols, and type aliases
//! that have generic type parameters. It holds the fully resolved where clause
//! constraints after the BIND phase.

use std::sync::Arc;

use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;
use crate::symbol::type_parameter::TypeParameterSymbol;
use crate::ty::WhereClause;

/// Behavior for symbols with generic type parameters.
///
/// This behavior stores:
/// - The type parameters declared on the symbol (e.g., `[T, U]`)
/// - The where clause with fully resolved protocol bounds (e.g., `where T: Equatable`)
///
/// Added during the BIND phase when protocol references in where clauses
/// have been resolved to actual protocol symbols.
#[derive(Debug, Clone)]
pub struct GenericsBehavior {
    /// Type parameters declared on this symbol
    type_parameters: Vec<Arc<TypeParameterSymbol>>,
    /// Where clause with resolved bounds
    where_clause: WhereClause,
}

impl GenericsBehavior {
    /// Create a new generics behavior with the given type parameters and where clause
    pub fn new(type_parameters: Vec<Arc<TypeParameterSymbol>>, where_clause: WhereClause) -> Self {
        Self {
            type_parameters,
            where_clause,
        }
    }

    /// Create an empty generics behavior (no type parameters, no constraints)
    pub fn empty() -> Self {
        Self {
            type_parameters: Vec::new(),
            where_clause: WhereClause::new(),
        }
    }

    /// Get the type parameters
    pub fn type_parameters(&self) -> &[Arc<TypeParameterSymbol>] {
        &self.type_parameters
    }

    /// Get the where clause
    pub fn where_clause(&self) -> &WhereClause {
        &self.where_clause
    }

    /// Check if this symbol is generic (has type parameters)
    pub fn is_generic(&self) -> bool {
        !self.type_parameters.is_empty()
    }

    /// Get the number of type parameters
    pub fn type_parameter_count(&self) -> usize {
        self.type_parameters.len()
    }
}

impl Behavior<KestrelLanguage> for GenericsBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::Generics
    }
}
