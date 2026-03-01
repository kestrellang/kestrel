//! Constraint-aware type assignability utilities (migrated from builder)
//!
//! Provides helpers to check assignability while considering where-clause
//! equality constraints in scope.

use kestrel_semantic_model::{ContextualOracle, SemanticModel, WhereClausesInScope};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_semantic_type_inference::TypeOracle;
use semantic_tree::symbol::{Symbol, SymbolId};

/// Check if `from` is assignable to `to`, considering equality constraints in scope.
pub fn is_assignable_with_constraints(
    from: &Ty,
    to: &Ty,
    model: &SemanticModel,
    context_id: SymbolId,
) -> bool {
    if from.is_assignable_to(to) {
        return true;
    }

    let oracle = ContextualOracle::new(model, context_id);

    // Check protocol conformance: a struct/enum can be assigned to a protocol it conforms to
    if let TyKind::Protocol { symbol, .. } = to.kind() {
        let protocol_id = symbol.metadata().id();
        if oracle.conforms_to(from, protocol_id) {
            return true;
        }
    }

    let where_clauses = model.query(WhereClausesInScope { context_id });
    if where_clauses.is_empty() {
        return false;
    }

    let from_normalized = oracle.normalize_with_constraints(from);
    let to_normalized = oracle.normalize_with_constraints(to);
    from_normalized.is_assignable_to(&to_normalized)
}
