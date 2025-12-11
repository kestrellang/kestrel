//! Constraint-aware type assignability checking
//!
//! This module provides type assignability checking that considers where clause
//! equality constraints. It walks up the parent chain to collect all applicable
//! constraints and uses them to determine if types are compatible.

use kestrel_semantic_tree::behavior_ext::SymbolBehaviorExt;
use kestrel_semantic_tree::ty::{Ty, TyKind, WhereClause};
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::database::Db;

/// Collect all where clauses from the context by walking up the parent chain.
///
/// This finds all type parameters in scope and their associated constraints,
/// including nested scopes like:
/// ```kestrel
/// struct Outer[T] where T: Iterator {
///     func inner[U](u: U) -> T.Item where U = T.Item {
///         return u
///     }
/// }
/// ```
pub fn collect_where_clauses(db: &dyn Db, context_id: SymbolId) -> Vec<WhereClause> {
    let mut clauses = Vec::new();
    let mut current_id = Some(context_id);

    while let Some(id) = current_id {
        let Some(symbol) = db.symbol_by_id(id) else {
            break;
        };

        // Check if this symbol has generics behavior (where clause)
        if let Some(generics_beh) = symbol.generics_behavior() {
            let wc = generics_beh.where_clause();
            if !wc.is_empty() {
                clauses.push(wc.clone());
            }
        }

        // Walk up to parent
        current_id = symbol.metadata().parent().map(|p| p.metadata().id());
    }

    clauses
}

/// Check if `from` type is assignable to `to` type, considering where clause
/// equality constraints from the given context.
///
/// This extends the basic `is_assignable_to` check by consulting equality constraints
/// like `T = U` or `T.Item = Int` that are in scope.
pub fn is_assignable_with_constraints(
    from: &Ty,
    to: &Ty,
    db: &dyn Db,
    context_id: SymbolId,
) -> bool {
    // First, try the basic assignability check
    if from.is_assignable_to(to) {
        return true;
    }

    // Collect all where clauses in scope
    let where_clauses = collect_where_clauses(db, context_id);

    // Collect all equality constraints
    let equalities: Vec<(&Ty, &Ty)> = where_clauses
        .iter()
        .flat_map(|wc| wc.equality_constraints())
        .collect();

    // If no equality constraints, fall back to basic check
    if equalities.is_empty() {
        return false;
    }

    // Normalize both types using equality constraints and check again
    let from_normalized = normalize_type(from, &equalities);
    let to_normalized = normalize_type(to, &equalities);

    from_normalized.is_assignable_to(&to_normalized)
}

/// Normalize a type by applying equality constraints.
///
/// For each equality constraint `L = R`, if the type matches `L`, replace it with `R`.
/// This handles both direct type parameter equalities (T = U) and associated type
/// equalities (T.Item = Int).
fn normalize_type(ty: &Ty, equalities: &[(&Ty, &Ty)]) -> Ty {
    // Check if this type matches any left side of an equality constraint
    for (left, right) in equalities {
        if types_match(ty, left) {
            // Found a matching constraint, return the right side
            return (*right).clone();
        }
        // Also check if types_match the right side (equality is symmetric)
        if types_match(ty, right) {
            return (*left).clone();
        }
    }

    // If no direct match, recursively normalize nested types
    match ty.kind() {
        TyKind::Tuple(elements) => {
            let normalized: Vec<Ty> = elements
                .iter()
                .map(|e| normalize_type(e, equalities))
                .collect();
            Ty::tuple(normalized, ty.span().clone())
        }
        TyKind::Array(element) => {
            let normalized = normalize_type(element, equalities);
            Ty::array(normalized, ty.span().clone())
        }
        TyKind::Function { params, return_type } => {
            let normalized_params: Vec<Ty> = params
                .iter()
                .map(|p| normalize_type(p, equalities))
                .collect();
            let normalized_return = normalize_type(return_type, equalities);
            Ty::function(normalized_params, normalized_return, ty.span().clone())
        }
        // For other types (primitives, structs, protocols, type params), return as-is
        _ => ty.clone(),
    }
}

/// Check if two types structurally match (for constraint lookup).
///
/// This is used to determine if a type matches the left side of an equality constraint.
/// It compares types by their structural identity (same type parameter ID, same
/// associated type path, etc.).
fn types_match(a: &Ty, b: &Ty) -> bool {
    match (a.kind(), b.kind()) {
        // Type parameters match if they have the same symbol ID
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        }
        // Associated types match if they have the same symbol ID and same container
        (
            TyKind::AssociatedType { symbol: a_sym, container: a_cont },
            TyKind::AssociatedType { symbol: b_sym, container: b_cont },
        ) => {
            if a_sym.metadata().id() != b_sym.metadata().id() {
                return false;
            }
            // Both must have the same container (or both None)
            match (a_cont, b_cont) {
                (Some(a_c), Some(b_c)) => types_match(a_c, b_c),
                (None, None) => true,
                _ => false,
            }
        }
        // For other types, use structural comparison via is_assignable_to
        // (which handles Never, Error, etc. correctly)
        _ => a.is_assignable_to(b) && b.is_assignable_to(a),
    }
}

#[cfg(test)]
mod tests {
    use kestrel_span::Span;
    use super::*;

    // Unit tests would require setting up a full semantic database,
    // so integration tests in lib/kestrel-test-suite are more appropriate.
}
