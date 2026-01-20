use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

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

    /// Get all constraints in this where clause
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Get all bounds for a specific type parameter
    pub fn bounds_for(&self, param_id: SymbolId) -> Vec<&Ty> {
        self.constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::TypeBound {
                    param: Some(id),
                    bounds,
                    ..
                } if *id == param_id => Some(bounds),
                Constraint::TypeBound { .. } => None,
                // Negative bounds don't contribute positive bounds
                Constraint::NegativeBound { .. } => None,
                // Inherited associated type bounds don't apply to type parameters
                Constraint::InheritedAssociatedTypeBound { .. } => None,
                // Type equality constraints don't contribute bounds
                Constraint::TypeEquality { .. } => None,
                // SelfBound constraints apply to Self, not type parameters
                Constraint::SelfBound { .. } => None,
            })
            .flatten()
            .collect()
    }

    /// Get all bounds for a specific type parameter along with the constraint's param span.
    ///
    /// Returns tuples of (bound, constraint_span) where constraint_span is the span
    /// of the type parameter name in the where clause (e.g., span of "F" in "F: Formattable").
    pub fn bounds_for_with_span(&self, param_id: SymbolId) -> Vec<(&Ty, &Span)> {
        self.constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::TypeBound {
                    param: Some(id),
                    bounds,
                    param_span,
                    ..
                } if *id == param_id => Some(bounds.iter().map(move |b| (b, param_span))),
                Constraint::TypeBound { .. } => None,
                Constraint::NegativeBound { .. } => None,
                Constraint::InheritedAssociatedTypeBound { .. } => None,
                Constraint::TypeEquality { .. } => None,
                Constraint::SelfBound { .. } => None,
            })
            .flatten()
            .collect()
    }

    /// Get all negative bounds for a specific type parameter
    pub fn negative_bounds_for(&self, param_id: SymbolId) -> Vec<&Ty> {
        self.constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::NegativeBound {
                    param: Some(id),
                    bound,
                    ..
                } if *id == param_id => Some(bound),
                _ => None,
            })
            .collect()
    }

    /// Check if a type parameter has a `not Copyable` bound
    pub fn has_not_copyable(&self, param_id: SymbolId) -> bool {
        self.constraints.iter().any(|c| match c {
            Constraint::NegativeBound {
                param: Some(id),
                bound,
                ..
            } if *id == param_id => {
                // Check if the bound is the Copyable protocol
                if let crate::ty::TyKind::Protocol { symbol, .. } = bound.kind() {
                    symbol.metadata().name().value == "Copyable"
                } else {
                    false
                }
            }
            _ => false,
        })
    }

    /// Get all equality constraints in this where clause
    pub fn equality_constraints(&self) -> Vec<(&Ty, &Ty)> {
        self.constraints
            .iter()
            .filter_map(|c| match c {
                Constraint::TypeEquality { left, right, .. } => Some((left, right)),
                _ => None,
            })
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
    /// A negative bound constraint: `T: not Copyable`
    ///
    /// This indicates that a type parameter does NOT satisfy a protocol bound.
    /// Currently only used for `not Copyable` to relax the implicit Copyable requirement.
    NegativeBound {
        /// The SymbolId of the type parameter being constrained.
        /// None if the type parameter name was not found in the declared parameters.
        param: Option<SymbolId>,
        /// The name of the type parameter as written in source (for error reporting)
        param_name: String,
        /// The span of the type parameter name (for error reporting)
        param_span: Span,
        /// The negated bound (the protocol this type does NOT need to satisfy)
        bound: Ty,
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
    /// A type equality constraint: `T.Item = Int` or `T = U`
    ///
    /// This constrains a type or associated type to be equal to another type.
    /// Used in where clauses: `where T.Item = Int, U = V`
    TypeEquality {
        /// The left side of the equality (type parameter or associated type path)
        left: Ty,
        /// The right side of the equality (the type it must equal)
        right: Ty,
        /// The span of the entire constraint
        span: Span,
    },
    /// A constraint on Self or Self's associated type in a protocol extension.
    ///
    /// Syntax: `Self: Protocol` or `Self.Item: Protocol`
    ///
    /// Used in protocol extension where clauses to constrain which conforming
    /// types receive the extension's methods.
    SelfBound {
        /// The path after Self (empty for `Self: Protocol`, non-empty for `Self.Item: Protocol`)
        /// For `Self.Item.SubItem: Protocol`, this would be `["Item", "SubItem"]`
        associated_type_path: Vec<String>,
        /// The span of the entire Self path (e.g., span of "Self.Item")
        path_span: Span,
        /// The bounds that must be satisfied (protocol types)
        bounds: Vec<Ty>,
    },
}

impl Constraint {
    /// Create a new type bound constraint with a resolved parameter
    pub fn type_bound(
        param: SymbolId,
        param_name: String,
        param_span: Span,
        bounds: Vec<Ty>,
    ) -> Self {
        Constraint::TypeBound {
            param: Some(param),
            param_name,
            param_span,
            bounds,
        }
    }

    /// Create a new type bound constraint with an unresolved (undeclared) parameter
    pub fn unresolved_type_bound(param_name: String, param_span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::TypeBound {
            param: None,
            param_name,
            param_span,
            bounds,
        }
    }

    /// Create an inherited associated type bound constraint
    ///
    /// Used for protocol where clauses like `Iterator.Item: Comparable`
    pub fn inherited_assoc_type_bound(path: String, span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::InheritedAssociatedTypeBound { path, span, bounds }
    }

    /// Create a type equality constraint
    ///
    /// Used for where clauses like `T.Item = Int` or `T = U`
    pub fn type_equality(left: Ty, right: Ty, span: Span) -> Self {
        Constraint::TypeEquality { left, right, span }
    }

    /// Create a negative bound constraint with a resolved parameter
    ///
    /// Used for where clauses like `T: not Copyable`
    pub fn negative_bound(
        param: SymbolId,
        param_name: String,
        param_span: Span,
        bound: Ty,
    ) -> Self {
        Constraint::NegativeBound {
            param: Some(param),
            param_name,
            param_span,
            bound,
        }
    }

    /// Create a negative bound constraint with an unresolved (undeclared) parameter
    pub fn unresolved_negative_bound(param_name: String, param_span: Span, bound: Ty) -> Self {
        Constraint::NegativeBound {
            param: None,
            param_name,
            param_span,
            bound,
        }
    }

    /// Create a Self bound constraint for protocol extensions
    ///
    /// Used for where clauses like `Self: Protocol` or `Self.Item: Protocol`
    pub fn self_bound(associated_type_path: Vec<String>, path_span: Span, bounds: Vec<Ty>) -> Self {
        Constraint::SelfBound {
            associated_type_path,
            path_span,
            bounds,
        }
    }

    /// Check if this is a Self bound constraint (for protocol extensions)
    pub fn is_self_bound(&self) -> bool {
        matches!(self, Constraint::SelfBound { .. })
    }

    /// Get the associated type path for a Self bound (empty for `Self: Protocol`)
    pub fn self_bound_associated_type_path(&self) -> Option<&[String]> {
        match self {
            Constraint::SelfBound {
                associated_type_path,
                ..
            } => Some(associated_type_path),
            _ => None,
        }
    }

    /// Get the type parameter this constraint applies to (if resolved)
    pub fn param_id(&self) -> Option<SymbolId> {
        match self {
            Constraint::TypeBound { param, .. } => *param,
            Constraint::NegativeBound { param, .. } => *param,
            Constraint::InheritedAssociatedTypeBound { .. } => None,
            Constraint::TypeEquality { .. } => None,
            // SelfBound doesn't reference a type parameter - it references Self
            Constraint::SelfBound { .. } => None,
        }
    }

    /// Get the type parameter name
    pub fn param_name(&self) -> &str {
        match self {
            Constraint::TypeBound { param_name, .. } => param_name,
            Constraint::NegativeBound { param_name, .. } => param_name,
            Constraint::InheritedAssociatedTypeBound { path, .. } => path,
            Constraint::TypeEquality { .. } => "",
            Constraint::SelfBound { .. } => "Self",
        }
    }

    /// Get the type parameter span
    pub fn param_span(&self) -> &Span {
        match self {
            Constraint::TypeBound { param_span, .. } => param_span,
            Constraint::NegativeBound { param_span, .. } => param_span,
            Constraint::InheritedAssociatedTypeBound { span, .. } => span,
            Constraint::TypeEquality { span, .. } => span,
            Constraint::SelfBound { path_span, .. } => path_span,
        }
    }

    /// Check if this constraint references an undeclared type parameter
    pub fn is_unresolved(&self) -> bool {
        match self {
            Constraint::TypeBound { param, .. } => param.is_none(),
            Constraint::NegativeBound { param, .. } => param.is_none(),
            // Inherited associated type bounds are always resolved (they've been validated)
            Constraint::InheritedAssociatedTypeBound { .. } => false,
            // Type equality constraints are always resolved
            Constraint::TypeEquality { .. } => false,
            // SelfBound is always "resolved" in the sense that Self is a known concept
            Constraint::SelfBound { .. } => false,
        }
    }

    /// Check if this is an inherited associated type bound
    pub fn is_inherited_assoc_type_bound(&self) -> bool {
        matches!(self, Constraint::InheritedAssociatedTypeBound { .. })
    }

    /// Check if this is a type equality constraint
    pub fn is_type_equality(&self) -> bool {
        matches!(self, Constraint::TypeEquality { .. })
    }

    /// Check if this is a negative bound constraint
    pub fn is_negative_bound(&self) -> bool {
        matches!(self, Constraint::NegativeBound { .. })
    }

    /// Get the type parameter ID this constraint applies to (if resolved)
    pub fn type_parameter_id(&self) -> Option<SymbolId> {
        match self {
            Constraint::TypeBound { param, .. } => *param,
            Constraint::NegativeBound { param, .. } => *param,
            _ => None,
        }
    }

    /// Get the bounds for this constraint (empty for non-type-bound constraints)
    pub fn bounds(&self) -> &[Ty] {
        match self {
            Constraint::TypeBound { bounds, .. } => bounds,
            Constraint::InheritedAssociatedTypeBound { bounds, .. } => bounds,
            Constraint::SelfBound { bounds, .. } => bounds,
            Constraint::NegativeBound { .. } => &[],
            Constraint::TypeEquality { .. } => &[],
        }
    }

    /// Get the negative bound for this constraint (None for non-negative-bound constraints)
    pub fn negative_bound_ty(&self) -> Option<&Ty> {
        match self {
            Constraint::NegativeBound { bound, .. } => Some(bound),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_span::Span;

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
        let bound = Ty::error(Span::new(0, 0..8));

        let constraint =
            Constraint::type_bound(param_id, "T".to_string(), Span::new(0, 0..1), vec![bound]);
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
        let bound = Ty::error(Span::new(0, 0..8));

        let constraint =
            Constraint::type_bound(param_id, "T".to_string(), Span::new(0, 0..1), vec![bound]);
        let wc = WhereClause::with_constraints(vec![constraint]);

        // Looking for bounds on a different param
        let bounds = wc.bounds_for(other_id);
        assert!(bounds.is_empty());
    }

    #[test]
    fn test_unresolved_constraint() {
        // Use error type as placeholder for protocol bound in test
        let bound = Ty::error(Span::new(0, 0..8));
        let constraint =
            Constraint::unresolved_type_bound("U".to_string(), Span::new(0, 0..1), vec![bound]);

        assert!(constraint.is_unresolved());
        assert_eq!(constraint.param_name(), "U");
        assert!(constraint.param_id().is_none());
    }
}
