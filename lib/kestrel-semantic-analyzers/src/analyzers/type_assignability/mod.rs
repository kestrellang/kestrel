//! Constraint-aware type assignability utilities (migrated from builder)
//!
//! Provides helpers to check assignability while considering where-clause
//! equality constraints in scope.

use kestrel_semantic_model::{SemanticModel, SymbolFor};
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind, WhereClause};
use semantic_tree::symbol::Symbol;
use semantic_tree::symbol::SymbolId;

/// Collect all where clauses from the context by walking up the parent chain.
pub fn collect_where_clauses(model: &SemanticModel, context_id: SymbolId) -> Vec<WhereClause> {
    let mut clauses = Vec::new();
    let mut current_id = Some(context_id);

    while let Some(id) = current_id {
        let Some(symbol) = model.query(SymbolFor { id }) else {
            break;
        };

        if let Some(generics_beh) = symbol.metadata().get_behavior::<GenericsBehavior>() {
            let wc = generics_beh.where_clause();
            if !wc.is_empty() {
                clauses.push(wc.clone());
            }
        }

        current_id = symbol.metadata().parent().map(|p| p.metadata().id());
    }

    clauses
}

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

    let where_clauses = collect_where_clauses(model, context_id);
    let equalities: Vec<(&Ty, &Ty)> = where_clauses
        .iter()
        .flat_map(|wc| wc.equality_constraints())
        .collect();

    if equalities.is_empty() {
        return false;
    }

    let from_normalized = normalize_type(from, &equalities);
    let to_normalized = normalize_type(to, &equalities);
    from_normalized.is_assignable_to(&to_normalized)
}

fn normalize_type(ty: &Ty, equalities: &[(&Ty, &Ty)]) -> Ty {
    for (left, right) in equalities {
        if types_match(ty, left) {
            return (*right).clone();
        }
        if types_match(ty, right) {
            return (*left).clone();
        }
    }
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
        TyKind::Function {
            params,
            return_type,
        } => {
            let normalized_params: Vec<Ty> = params
                .iter()
                .map(|p| normalize_type(p, equalities))
                .collect();
            let normalized_return = normalize_type(return_type, equalities);
            Ty::function(normalized_params, normalized_return, ty.span().clone())
        }
        _ => ty.clone(),
    }
}

fn types_match(a: &Ty, b: &Ty) -> bool {
    match (a.kind(), b.kind()) {
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        }
        (
            TyKind::AssociatedType {
                symbol: a_sym,
                container: a_cont,
            },
            TyKind::AssociatedType {
                symbol: b_sym,
                container: b_cont,
            },
        ) => {
            if a_sym.metadata().id() != b_sym.metadata().id() {
                return false;
            }
            match (a_cont, b_cont) {
                (Some(a_c), Some(b_c)) => types_match(a_c, b_c),
                (None, None) => true,
                _ => false,
            }
        }
        _ => a.is_assignable_to(b) && b.is_assignable_to(a),
    }
}
