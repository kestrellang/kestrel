//! Constraint-aware type assignability utilities (migrated from builder)
//!
//! Provides helpers to check assignability while considering where-clause
//! equality constraints in scope.

use kestrel_semantic_model::{SemanticModel, SymbolFor};
use kestrel_semantic_tree::behavior::extension_target::ExtensionTargetBehavior;
use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind, WhereClause};
use kestrel_semantic_type_inference::TypeOracle;
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

        if let Some(target_beh) = symbol.metadata().get_behavior::<ExtensionTargetBehavior>() {
            let wc = target_beh.where_clause();
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

    // Check protocol conformance: a struct/enum can be assigned to a protocol it conforms to
    if let TyKind::Protocol { symbol, .. } = to.kind() {
        let protocol_id = symbol.metadata().id();
        if model.conforms_to(from, protocol_id) {
            return true;
        }
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
    let mut current = ty.clone();
    let mut seen = std::collections::HashSet::new();
    seen.insert(current.to_string());

    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 10; // Safety cap

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        // Try to apply equality constraints to the whole type
        for (left, right) in equalities {
            let matches_left = types_match(&current, left);
            let matches_right = types_match(&current, right);

            if matches_left || matches_right {
                // We have a match. We want to pick a canonical representative.
                // Rule: prefer the one that is "more concrete" or smaller by string.
                let next = if is_more_concrete(left, right) {
                    (*left).clone()
                } else {
                    (*right).clone()
                };

                let next_str = next.to_string();
                if next_str != current.to_string() && !seen.contains(&next_str) {
                    current = next;
                    seen.insert(next_str);
                    changed = true;
                    break;
                }
            }
        }

        if changed {
            continue;
        }

        // Try to normalize components
        match current.kind().clone() {
            TyKind::Tuple(elements) => {
                let mut new_elements = Vec::new();
                let mut inner_changed = false;
                for e in elements {
                    let normalized = normalize_type(&e, equalities);
                    if normalized.to_string() != e.to_string() {
                        inner_changed = true;
                    }
                    new_elements.push(normalized);
                }
                if inner_changed {
                    current = Ty::tuple(new_elements, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Array(element) => {
                let normalized = normalize_type(&element, equalities);
                if normalized.to_string() != element.to_string() {
                    current = Ty::array(normalized, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::Function {
                params,
                return_type,
            } => {
                let mut new_params = Vec::new();
                let mut inner_changed = false;
                for p in params {
                    let normalized = normalize_type(&p, equalities);
                    if normalized.to_string() != p.to_string() {
                        inner_changed = true;
                    }
                    new_params.push(normalized);
                }
                let normalized_return = normalize_type(&return_type, equalities);
                if normalized_return.to_string() != return_type.to_string() {
                    inner_changed = true;
                }
                if inner_changed {
                    current = Ty::function(new_params, normalized_return, current.span().clone());
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            TyKind::AssociatedType { symbol, container } if container.is_some() => {
                let cont = container.as_ref().unwrap();
                let normalized_container = normalize_type(cont, equalities);
                if normalized_container.to_string() != cont.to_string() {
                    current = Ty::qualified_associated_type(
                        symbol.clone(),
                        normalized_container,
                        current.span().clone(),
                    );
                    let current_str = current.to_string();
                    if !seen.contains(&current_str) {
                        seen.insert(current_str);
                        changed = true;
                    }
                }
            },
            _ => {},
        }
    }

    current
}

fn is_more_concrete(a: &Ty, b: &Ty) -> bool {
    let a_score = type_score(a);
    let b_score = type_score(b);
    if a_score != b_score {
        a_score > b_score
    } else {
        // Tie-breaker: use Display string
        a.to_string() < b.to_string()
    }
}

fn type_score(ty: &Ty) -> i32 {
    match ty.kind() {
        TyKind::TypeParameter(_) => 0,
        TyKind::AssociatedType { .. } => 1,
        TyKind::SelfType => 2,
        TyKind::Protocol { .. } => 3,
        TyKind::Struct { .. } => 4,
        TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String | TyKind::Unit => 5,
        TyKind::Tuple(_) | TyKind::Array(_) | TyKind::Function { .. } => 4, // Complex but concrete-ish
        _ => -1,
    }
}

fn types_match(a: &Ty, b: &Ty) -> bool {
    match (a.kind(), b.kind()) {
        (TyKind::TypeParameter(a_param), TyKind::TypeParameter(b_param)) => {
            a_param.metadata().id() == b_param.metadata().id()
        },
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
        },
        // If one is a type param/associated type and other isn't, they don't match
        (TyKind::TypeParameter(_), _) | (_, TyKind::TypeParameter(_)) => false,
        (TyKind::AssociatedType { .. }, _) | (_, TyKind::AssociatedType { .. }) => false,

        _ => a.is_assignable_to(b) && b.is_assignable_to(a),
    }
}
