//! Constraint solver using unification and fixpoint iteration.
//!
//! The solver processes constraints in rounds until no more progress can be made.
//! Each round attempts to solve all pending constraints. Constraints that cannot
//! be solved yet (because their types aren't resolved) are deferred to the next round.

use std::collections::HashSet;

use kestrel_semantic_tree::ty::{Ty, TyId, TyKind};
use kestrel_span::Span;

use crate::constraint::Constraint;
use crate::context::InferenceContext;
use crate::error::InferenceError;
use crate::oracle::MemberError;
use crate::solution::{Solution, ValueResolution};

/// Result of attempting to solve a single constraint.
enum SolveResult {
    /// Constraint was fully solved (may have produced substitutions)
    Solved,
    /// Constraint couldn't be solved yet (types not resolved enough)
    Deferred,
}

/// Solve all constraints in the context and return a solution.
///
/// Errors are accumulated in the solution rather than failing fast,
/// allowing multiple type errors to be reported in a single pass.
pub fn solve(mut ctx: InferenceContext<'_>) -> Solution {
    // Iterate until fixpoint (no progress)
    loop {
        let progress = solve_round(&mut ctx);
        if !progress {
            break;
        }
    }

    // Check that everything was resolved, add error if not
    check_fully_resolved(&mut ctx);

    ctx.into_solution()
}

/// Run one round of constraint solving.
///
/// Returns true if any progress was made (i.e., at least one constraint was solved
/// or a new substitution was added).
fn solve_round(ctx: &mut InferenceContext<'_>) -> bool {
    let mut progress = false;
    let constraints = ctx.take_constraints();

    for constraint in constraints {
        match try_solve(ctx, &constraint) {
            Ok(SolveResult::Solved) => progress = true,
            Ok(SolveResult::Deferred) => ctx.push_constraint(constraint),
            Err(error) => {
                // Accumulate error and mark as progress (constraint was processed)
                ctx.add_error(error);
                progress = true;
            }
        }
    }

    progress
}

/// Attempt to solve a single constraint.
fn try_solve(
    ctx: &mut InferenceContext<'_>,
    constraint: &Constraint,
) -> Result<SolveResult, InferenceError> {
    match constraint {
        Constraint::Equals { a, b, span } => unify(ctx, *a, *b, span),
        Constraint::Conforms { ty, protocol } => check_conforms(ctx, *ty, protocol),
        Constraint::Normalizes {
            base,
            assoc_name,
            result,
            span,
        } => normalize(ctx, *base, assoc_name, *result, span),
        Constraint::MemberAccess {
            receiver,
            member,
            is_static,
            result,
            expr_id,
            span,
        } => resolve_member(ctx, *receiver, member, *is_static, *result, *expr_id, span),
    }
}

/// Unify two types, producing substitutions that make them equal.
fn unify(
    ctx: &mut InferenceContext<'_>,
    a: TyId,
    b: TyId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    // Get the current resolved types for both IDs
    let ty_a = resolve_type(ctx, a);
    let ty_b = resolve_type(ctx, b);

    // If both are the same TyId (after resolution), they're already unified
    if ty_a.id() == ty_b.id() {
        return Ok(SolveResult::Solved);
    }

    // Handle inference placeholders
    match (ty_a.kind(), ty_b.kind()) {
        // Both are inference placeholders - unify them
        (TyKind::Infer, TyKind::Infer) => {
            // Map one to the other
            ctx.substitutions_mut().insert(ty_a.id(), ty_b.clone());
            Ok(SolveResult::Solved)
        }

        // One is an inference placeholder - substitute it
        (TyKind::Infer, _) => {
            if occurs_check(ty_a.id(), &ty_b, ctx) {
                return Err(InferenceError::occurs_check(ty_a.id(), ty_b.clone(), span.clone()));
            }
            ctx.substitutions_mut().insert(ty_a.id(), ty_b.clone());
            Ok(SolveResult::Solved)
        }
        (_, TyKind::Infer) => {
            if occurs_check(ty_b.id(), &ty_a, ctx) {
                return Err(InferenceError::occurs_check(ty_b.id(), ty_a.clone(), span.clone()));
            }
            ctx.substitutions_mut().insert(ty_b.id(), ty_a.clone());
            Ok(SolveResult::Solved)
        }

        // Error types unify with anything (suppress cascading errors)
        (TyKind::Error, _) | (_, TyKind::Error) => Ok(SolveResult::Solved),

        // Never unifies with anything (bottom type)
        (TyKind::Never, _) | (_, TyKind::Never) => Ok(SolveResult::Solved),

        // Structural unification for compound types
        (TyKind::Tuple(elems_a), TyKind::Tuple(elems_b)) => {
            if elems_a.len() != elems_b.len() {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }
            for (ea, eb) in elems_a.iter().zip(elems_b.iter()) {
                ctx.equate(ea.id(), eb.id(), span.clone());
            }
            Ok(SolveResult::Solved)
        }

        (TyKind::Array(elem_a), TyKind::Array(elem_b)) => {
            ctx.equate(elem_a.id(), elem_b.id(), span.clone());
            Ok(SolveResult::Solved)
        }

        (
            TyKind::Function {
                params: params_a,
                return_type: ret_a,
            },
            TyKind::Function {
                params: params_b,
                return_type: ret_b,
            },
        ) => {
            if params_a.len() != params_b.len() {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }
            for (pa, pb) in params_a.iter().zip(params_b.iter()) {
                ctx.equate(pa.id(), pb.id(), span.clone());
            }
            ctx.equate(ret_a.id(), ret_b.id(), span.clone());
            Ok(SolveResult::Solved)
        }

        // Nominal types - check symbol equality and unify type arguments
        (
            TyKind::Struct {
                symbol: sym_a,
                substitutions: subs_a,
            },
            TyKind::Struct {
                symbol: sym_b,
                substitutions: subs_b,
            },
        ) => {
            use semantic_tree::symbol::Symbol;
            use kestrel_semantic_tree::language::KestrelLanguage;

            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

            if id_a != id_b {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }

            // Unify substitutions
            for ((_, sub_a), (_, sub_b)) in subs_a.iter().zip(subs_b.iter()) {
                ctx.equate(sub_a.id(), sub_b.id(), span.clone());
            }
            Ok(SolveResult::Solved)
        }

        (
            TyKind::Protocol {
                symbol: sym_a,
                substitutions: subs_a,
            },
            TyKind::Protocol {
                symbol: sym_b,
                substitutions: subs_b,
            },
        ) => {
            use semantic_tree::symbol::Symbol;
            use kestrel_semantic_tree::language::KestrelLanguage;

            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

            if id_a != id_b {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }

            for ((_, sub_a), (_, sub_b)) in subs_a.iter().zip(subs_b.iter()) {
                ctx.equate(sub_a.id(), sub_b.id(), span.clone());
            }
            Ok(SolveResult::Solved)
        }

        // Type parameters - only equal if they're the same parameter
        (TyKind::TypeParameter(param_a), TyKind::TypeParameter(param_b)) => {
            use semantic_tree::symbol::Symbol;
            use kestrel_semantic_tree::language::KestrelLanguage;

            let id_a = Symbol::<KestrelLanguage>::metadata(param_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(param_b.as_ref()).id();

            if id_a == id_b {
                Ok(SolveResult::Solved)
            } else {
                Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ))
            }
        }

        // Associated types - defer if not yet resolved
        (TyKind::AssociatedType { .. }, _) | (_, TyKind::AssociatedType { .. }) => {
            // Associated types need to be normalized first
            Ok(SolveResult::Deferred)
        }

        // Self type matches Self or compatible struct/protocol
        (TyKind::SelfType, TyKind::SelfType) => Ok(SolveResult::Solved),
        (TyKind::SelfType, TyKind::Struct { .. }) | (TyKind::Struct { .. }, TyKind::SelfType) => {
            Ok(SolveResult::Solved)
        }
        (TyKind::SelfType, TyKind::Protocol { .. }) | (TyKind::Protocol { .. }, TyKind::SelfType) => {
            Ok(SolveResult::Solved)
        }

        // Primitive types - exact match required
        (TyKind::Unit, TyKind::Unit) => Ok(SolveResult::Solved),
        (TyKind::Bool, TyKind::Bool) => Ok(SolveResult::Solved),
        (TyKind::String, TyKind::String) => Ok(SolveResult::Solved),
        (TyKind::Int(bits_a), TyKind::Int(bits_b)) if bits_a == bits_b => Ok(SolveResult::Solved),
        (TyKind::Float(bits_a), TyKind::Float(bits_b)) if bits_a == bits_b => Ok(SolveResult::Solved),

        // Type aliases - expand and retry
        (TyKind::TypeAlias { .. }, _) => {
            let expanded = ctx.oracle().expand_type_alias(&ty_a);
            ctx.equate(expanded.id(), ty_b.id(), span.clone());
            Ok(SolveResult::Solved)
        }
        (_, TyKind::TypeAlias { .. }) => {
            let expanded = ctx.oracle().expand_type_alias(&ty_b);
            ctx.equate(ty_a.id(), expanded.id(), span.clone());
            Ok(SolveResult::Solved)
        }

        // No match - type mismatch
        _ => Err(InferenceError::type_mismatch(
            ty_a.clone(),
            ty_b.clone(),
            span.clone(),
        )),
    }
}

/// Check if a type conforms to a protocol.
fn check_conforms(
    ctx: &mut InferenceContext<'_>,
    ty: TyId,
    protocol: &crate::constraint::ProtocolRef,
) -> Result<SolveResult, InferenceError> {
    let resolved = resolve_type(ctx, ty);

    // If the type is still an inference placeholder, defer
    if matches!(resolved.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Check conformance via the oracle
    if ctx.oracle().conforms_to(&resolved, protocol.symbol_id) {
        Ok(SolveResult::Solved)
    } else {
        // TODO: Get protocol name from oracle
        Err(InferenceError::conformance_failure(
            resolved.clone(),
            format!("{:?}", protocol.symbol_id),
            protocol.span.clone(),
        ))
    }
}

/// Resolve an associated type projection.
fn normalize(
    ctx: &mut InferenceContext<'_>,
    base: TyId,
    assoc_name: &str,
    result: TyId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let base_ty = resolve_type(ctx, base);

    // If the base type is still an inference placeholder, defer
    if matches!(base_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Try to resolve the associated type via the oracle
    match ctx.oracle().resolve_associated_type(&base_ty, assoc_name) {
        Some(resolved_assoc) => {
            // Register and unify with the result
            ctx.register_type(&resolved_assoc);
            ctx.equate(resolved_assoc.id(), result, span.clone());
            Ok(SolveResult::Solved)
        }
        None => Err(InferenceError::associated_type_not_found(
            base_ty.clone(),
            assoc_name.to_string(),
            span.clone(),
        )),
    }
}

/// Resolve a member access.
fn resolve_member(
    ctx: &mut InferenceContext<'_>,
    receiver: TyId,
    member: &str,
    is_static: bool,
    result: TyId,
    expr_id: kestrel_semantic_tree::expr::ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let receiver_ty = resolve_type(ctx, receiver);

    // If the receiver type is still an inference placeholder, defer
    if matches!(receiver_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Try to resolve the member via the oracle
    match ctx.oracle().resolve_member(&receiver_ty, member, is_static) {
        Ok(resolution) => {
            // Record the value resolution
            ctx.values_mut().insert(
                expr_id,
                ValueResolution::new(resolution.symbol_id, resolution.substitutions),
            );

            // Register and unify the member type with the result
            ctx.register_type(&resolution.ty);
            ctx.equate(resolution.ty.id(), result, span.clone());
            Ok(SolveResult::Solved)
        }
        Err(MemberError::UnknownType) => {
            // Shouldn't happen since we checked for Infer above, but defer anyway
            Ok(SolveResult::Deferred)
        }
        Err(MemberError::NotFound { .. }) => Err(InferenceError::member_not_found(
            receiver_ty.clone(),
            member.to_string(),
            span.clone(),
        )),
        Err(MemberError::Ambiguous { count }) => Err(InferenceError::internal(format!(
            "ambiguous member '{}': {} candidates",
            member, count
        ))),
    }
}

/// Follow the substitution chain to get the current resolved type for an ID.
fn resolve_type(ctx: &InferenceContext<'_>, id: TyId) -> Ty {
    // Follow substitution chain
    let mut current_id = id;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current_id) {
            // Cycle detected - return what we have
            break;
        }

        if let Some(subst) = ctx.substitutions().get(&current_id) {
            current_id = subst.id();
        } else {
            break;
        }
    }

    // Return the substituted type if available, otherwise look in registry
    ctx.substitutions()
        .get(&current_id)
        .cloned()
        .or_else(|| ctx.type_registry().get(&current_id).cloned())
        .unwrap_or_else(|| {
            // If not found anywhere, create an inference placeholder
            Ty::infer(Span::from(0..0))
        })
}

/// Check if var occurs in ty (prevents infinite types).
fn occurs_check(var: TyId, ty: &Ty, ctx: &InferenceContext<'_>) -> bool {
    occurs_check_inner(var, ty, ctx, &mut HashSet::new())
}

fn occurs_check_inner(
    var: TyId,
    ty: &Ty,
    ctx: &InferenceContext<'_>,
    visited: &mut HashSet<TyId>,
) -> bool {
    // Check if we've already visited this type (cycle prevention)
    if !visited.insert(ty.id()) {
        return false;
    }

    // Check if this is the variable we're looking for
    if ty.id() == var {
        return true;
    }

    // If this type has a substitution, check that too
    if let Some(subst) = ctx.substitutions().get(&ty.id()) {
        if occurs_check_inner(var, subst, ctx, visited) {
            return true;
        }
    }

    // Recursively check compound types
    match ty.kind() {
        TyKind::Tuple(elements) => elements
            .iter()
            .any(|e| occurs_check_inner(var, e, ctx, visited)),
        TyKind::Array(elem) => occurs_check_inner(var, elem, ctx, visited),
        TyKind::Function {
            params,
            return_type,
        } => {
            params
                .iter()
                .any(|p| occurs_check_inner(var, p, ctx, visited))
                || occurs_check_inner(var, return_type, ctx, visited)
        }
        TyKind::Struct { substitutions, .. }
        | TyKind::Protocol { substitutions, .. }
        | TyKind::TypeAlias { substitutions, .. } => substitutions
            .iter()
            .any(|(_, t)| occurs_check_inner(var, t, ctx, visited)),
        TyKind::AssociatedType { container, .. } => {
            container
                .as_ref()
                .map(|c| occurs_check_inner(var, c, ctx, visited))
                .unwrap_or(false)
        }
        // Leaf types
        _ => false,
    }
}

/// Check that all inference placeholders have been resolved.
/// If any remain unresolved, adds an Ambiguous error to the context.
fn check_fully_resolved(ctx: &mut InferenceContext<'_>) {
    let mut unresolved = Vec::new();

    // Check all registered types
    for (id, ty) in ctx.type_registry() {
        if matches!(ty.kind(), TyKind::Infer) && !ctx.substitutions().contains_key(id) {
            unresolved.push(*id);
        }
    }

    // Check for any inference placeholders in remaining constraints
    for constraint in ctx.constraints() {
        match constraint {
            Constraint::Equals { a, b, .. } => {
                check_resolved_id(*a, ctx, &mut unresolved);
                check_resolved_id(*b, ctx, &mut unresolved);
            }
            Constraint::Conforms { ty, .. } => {
                check_resolved_id(*ty, ctx, &mut unresolved);
            }
            Constraint::Normalizes { base, result, .. } => {
                check_resolved_id(*base, ctx, &mut unresolved);
                check_resolved_id(*result, ctx, &mut unresolved);
            }
            Constraint::MemberAccess {
                receiver, result, ..
            } => {
                check_resolved_id(*receiver, ctx, &mut unresolved);
                check_resolved_id(*result, ctx, &mut unresolved);
            }
        }
    }

    if !unresolved.is_empty() {
        // Deduplicate
        unresolved.sort_by_key(|id| id.raw());
        unresolved.dedup();
        ctx.add_error(InferenceError::ambiguous(unresolved));
    }
}

fn check_resolved_id(id: TyId, ctx: &InferenceContext<'_>, unresolved: &mut Vec<TyId>) {
    let ty = resolve_type(ctx, id);
    if matches!(ty.kind(), TyKind::Infer) {
        unresolved.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracle::{MemberError, MemberResolution, TypeOracle};
    use kestrel_semantic_tree::ty::Ty;
    use semantic_tree::symbol::SymbolId;

    struct TestOracle;

    impl TypeOracle for TestOracle {
        fn resolve_member(
            &self,
            _receiver_ty: &Ty,
            _member: &str,
            _is_static: bool,
        ) -> Result<MemberResolution, MemberError> {
            Err(MemberError::NotFound {
                receiver_ty: Ty::unit(Span::from(0..0)),
                member: String::new(),
            })
        }

        fn conforms_to(&self, _ty: &Ty, _protocol_id: SymbolId) -> bool {
            false
        }

        fn resolve_associated_type(&self, _container: &Ty, _assoc_name: &str) -> Option<Ty> {
            None
        }
    }

    #[test]
    fn test_unify_same_primitive() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let ty1 = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::from(0..3));
        let ty2 = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::from(4..7));

        ctx.register_type(&ty1);
        ctx.register_type(&ty2);
        ctx.equate(ty1.id(), ty2.id(), Span::from(0..7));

        let solution = ctx.solve();
        assert!(!solution.has_errors());
    }

    #[test]
    fn test_unify_infer_with_concrete() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let infer_ty = Ty::infer(Span::from(0..1));
        let concrete_ty = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::from(2..5));

        ctx.register_type(&infer_ty);
        ctx.register_type(&concrete_ty);
        ctx.equate(infer_ty.id(), concrete_ty.id(), Span::from(0..5));

        let solution = ctx.solve();
        assert!(!solution.has_errors());

        let resolved = solution.get_type(infer_ty.id());
        assert!(resolved.is_some());
        assert!(resolved.unwrap().is_int());
    }

    #[test]
    fn test_unify_mismatched_types() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let int_ty = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::from(0..3));
        let string_ty = Ty::string(Span::from(4..10));

        ctx.register_type(&int_ty);
        ctx.register_type(&string_ty);
        ctx.equate(int_ty.id(), string_ty.id(), Span::from(0..10));

        let solution = ctx.solve();
        assert!(solution.has_errors());
        assert!(matches!(
            &solution.errors()[0],
            InferenceError::TypeMismatch { .. }
        ));
    }
}
