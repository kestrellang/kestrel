use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use super::Ty;

/// Represents a where clause containing type constraints.
///
/// Where clauses appear on generic containers (structs, functions, protocols, type aliases)
/// and specify bounds that type parameters must satisfy.
///
/// Example: `where T: Comparable[T] and Hashable, U: Display`
#[derive(Debug, Clone, Default)]
pub struct WhereClause {
    pub constraints: Vec<Constraint>,
}

impl WhereClause {
    /// Create an empty where clause
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// Create a where clause with constraints
    pub fn with_constraints(constraints: Vec<Constraint>) -> Self {
        Self { constraints }
    }

    /// Check if the where clause has no constraints
    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Add a constraint to the where clause
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Get all bounds for a specific type parameter
    pub fn bounds_for(&self, param_id: SymbolId) -> Vec<&Ty> {
        self.constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::TypeBound { param: Some(id), bounds, .. } if *id == param_id => Some(bounds),
                Constraint::TypeBound { .. } => None,
                // Inherited associated type bounds don't apply to type parameters
                Constraint::InheritedAssociatedTypeBound { .. } => None,
            })
            .flatten()
            .collect()
    }
}

/// A single constraint in a where clause.
#[derive(Debug, Clone)]
pub enum Constraint {
    /// A type bound constraint: `T: Protocol and Protocol2`
    ///
    /// The `param` is the SymbolId of the type parameter being constrained (None if undeclared).
    /// The `bounds` are the types that the parameter must satisfy (typically protocols,
    /// but can be generic protocol instantiations like `Iterator[Int]`).
    TypeBound {
        /// The SymbolId of the type parameter being constrained.
        /// None if the type parameter name was not found in the declared parameters.
        param: Option<SymbolId>,
        /// The name of the type parameter as written in source (for error reporting)
        param_name: String,
        /// The span of the type parameter name (for error reporting)
        param_span: Span,
        /// The bounds that the type parameter must satisfy
        bounds: Vec<Ty>,
    },
    /// A constraint on an inherited protocol's associated type: `Iterator.Item: Comparable`
    ///
    /// This is used in protocol declarations to constrain associated types from parent protocols.
    /// Example: `protocol SortedIterator: Iterator where Iterator.Item: Comparable { }`
    InheritedAssociatedTypeBound {
        /// The full path name (e.g., "Iterator.Item")
        path: String,
        /// The span of the path
        span: Span,
        /// The bounds that the associated type must satisfy
        bounds: Vec<Ty>,
    },
    // Future: TypeEquality for associated types
    // TypeEquality { left: TypePath, right: Ty }
}

impl Constraint {
    /// Create a new type bound constraint with a resolved parameter
    pub fn type_bound(param: SymbolId, param_name: String, param_span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::TypeBound { param: Some(param), param_name, param_span, bounds }
    }

    /// Create a new type bound constraint with an unresolved (undeclared) parameter
    pub fn unresolved_type_bound(param_name: String, param_span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::TypeBound { param: None, param_name, param_span, bounds }
    }

    /// Create an inherited associated type bound constraint
    ///
    /// Used for protocol where clauses like `Iterator.Item: Comparable`
    pub fn inherited_assoc_type_bound(path: String, span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::InheritedAssociatedTypeBound { path, span, bounds }
    }

    /// Get the type parameter this constraint applies to (if resolved)
    pub fn param_id(&self) -> Option<SymbolId> {
        match self {
            Constraint::TypeBound { param, .. } => *param,
            Constraint::InheritedAssociatedTypeBound { .. } => None,
        }
    }

    /// Get the type parameter name
    pub fn param_name(&self) -> &str {
        match self {
            Constraint::TypeBound { param_name, .. } => param_name,
            Constraint::InheritedAssociatedTypeBound { path, .. } => path,
        }
    }

    /// Get the type parameter span
    pub fn param_span(&self) -> &Span {
        match self {
            Constraint::TypeBound { param_span, .. } => param_span,
            Constraint::InheritedAssociatedTypeBound { span, .. } => span,
        }
    }

    /// Check if this constraint references an undeclared type parameter
    pub fn is_unresolved(&self) -> bool {
        match self {
            Constraint::TypeBound { param, .. } => param.is_none(),
            // Inherited associated type bounds are always resolved (they've been validated)
            Constraint::InheritedAssociatedTypeBound { .. } => false,
        }
    }

    /// Check if this is an inherited associated type bound
    pub fn is_inherited_assoc_type_bound(&self) -> bool {
        matches!(self, Constraint::InheritedAssociatedTypeBound { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_where_clause() {
        let wc = WhereClause::new();
        assert!(wc.is_empty());
        assert!(wc.constraints.is_empty());
    }

    #[test]
    fn test_where_clause_with_constraints() {
        let param_id = SymbolId::new();
        // Use error type as placeholder for protocol bound in test
        let bound = Ty::error(0..8);

        let constraint = Constraint::type_bound(param_id, "T".to_string(), 0..1, vec![bound]);
        let wc = WhereClause::with_constraints(vec![constraint]);

        assert!(!wc.is_empty());
        assert_eq!(wc.constraints.len(), 1);

        let bounds = wc.bounds_for(param_id);
        assert_eq!(bounds.len(), 1);
    }

    #[test]
    fn test_bounds_for_unknown_param() {
        let param_id = SymbolId::new();
        let other_id = SymbolId::new();
        // Use error type as placeholder for protocol bound in test
        let bound = Ty::error(0..8);

        let constraint = Constraint::type_bound(param_id, "T".to_string(), 0..1, vec![bound]);
        let wc = WhereClause::with_constraints(vec![constraint]);

        // Looking for bounds on a different param
        let bounds = wc.bounds_for(other_id);
        assert!(bounds.is_empty());
    }

    #[test]
    fn test_unresolved_constraint() {
        // Use error type as placeholder for protocol bound in test
        let bound = Ty::error(0..8);
        let constraint = Constraint::unresolved_type_bound("U".to_string(), 0..1, vec![bound]);

        assert!(constraint.is_unresolved());
        assert_eq!(constraint.param_name(), "U");
        assert!(constraint.param_id().is_none());
    }
}
