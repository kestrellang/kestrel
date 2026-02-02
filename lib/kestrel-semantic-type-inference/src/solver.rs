//! Constraint solver using unification and fixpoint iteration.
//!
//! The solver processes constraints in rounds until no more progress can be made.
//! Each round attempts to solve all pending constraints. Constraints that cannot
//! be solved yet (because their types aren't resolved) are deferred to the next round.

// InferenceError contains spans and strings, making it large. Boxing would add
// complexity for little benefit since errors are rare.
#![allow(clippy::result_large_err)]

use std::collections::HashSet;

use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::ty::{ParamInfo, Ty, TyId, TyKind};
use kestrel_span::Span;

use crate::constraint::Constraint;
use crate::context::InferenceContext;
use crate::error::InferenceError;
use crate::oracle::MemberError;
use crate::solution::{PromotionInfo, Solution, ValueResolution};

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
    // Pre-scan constraints to identify literal types that have default types
    // (integer, float, string, bool, char). We need to defer Promotable constraints
    // for these literals until their defaults are applied.
    // Note: Null and array literals do NOT have defaults and need type context,
    // so we don't mark them - they should unify immediately.
    let literal_ty_ids: Vec<TyId> = ctx
        .constraints()
        .iter()
        .filter_map(|constraint| {
            let Constraint::Conforms { ty, protocol } = constraint else {
                return None;
            };
            let feature = get_literal_feature_for_protocol(&ctx, protocol.symbol_id)?;
            // Only mark literals that have defaults
            match feature {
                LanguageFeature::ExpressibleByIntLiteral
                | LanguageFeature::ExpressibleByFloatLiteral
                | LanguageFeature::ExpressibleByStringLiteral
                | LanguageFeature::ExpressibleByBoolLiteral
                | LanguageFeature::ExpressibleByCharLiteral => Some(*ty),
                // Null, array, and dictionary literals need context, don't defer
                LanguageFeature::ExpressibleByNullLiteral
                | LanguageFeature::_ExpressibleByArrayLiteral
                | LanguageFeature::_ExpressibleByDictionaryLiteral => None,
                _ => None,
            }
        })
        .collect();

    for ty_id in literal_ty_ids {
        ctx.mark_literal_ty(ty_id);
    }

    // Iterate until fixpoint (no progress)
    loop {
        let progress = solve_round(&mut ctx);
        if !progress {
            break;
        }
    }

    // Apply default types for unresolved literal types.
    // Loop until no more defaults can be applied - this handles nested structures
    // like [[1, 2], [3, 4]] where inner arrays need defaults before outer arrays.
    loop {
        let defaults_applied = apply_default_literal_types(&mut ctx);
        if !defaults_applied {
            break;
        }

        // Run solving rounds after each default application
        loop {
            let progress = solve_round(&mut ctx);
            if !progress {
                break;
            }
        }
    }

    // Check that everything was resolved, add error if not
    check_fully_resolved(&mut ctx);

    ctx.into_solution()
}

/// Apply default types for unresolved literal types.
///
/// When a literal's type is still ambiguous (Infer) after the main solving loop,
/// we apply the default type for that literal kind:
/// - Integer literals → Int64 (configurable via DefaultIntegerLiteralType)
/// - Float literals → Float64 (configurable via DefaultFloatLiteralType)
/// - String literals → String
/// - Bool literals → Bool
/// - Array literals → Array[ElementType] where ElementType is resolved from elements
///
/// Returns true if any defaults were applied, false otherwise.
fn apply_default_literal_types(ctx: &mut InferenceContext<'_>) -> bool {
    // Collect literal constraints where the type is still Infer
    let constraints = ctx.take_constraints();
    let mut literals_to_default: Vec<(TyId, LanguageFeature, Span)> = Vec::new();

    // Also collect Normalizes constraints for array element types
    let mut array_element_types: std::collections::HashMap<TyId, TyId> =
        std::collections::HashMap::new();

    for constraint in &constraints {
        if let Constraint::Conforms { ty, protocol } = constraint {
            let resolved = resolve_type(ctx, *ty);
            if matches!(resolved.kind(), TyKind::Infer) {
                // Check if this is a literal protocol
                if let Some(feature) = get_literal_feature_for_protocol(ctx, protocol.symbol_id) {
                    literals_to_default.push((*ty, feature, protocol.span.clone()));
                }
            }
        }
        // Track array element type associations from Normalizes constraints
        if let Constraint::Normalizes {
            base,
            assoc_name,
            result,
            ..
        } = constraint
            && assoc_name == "Element"
        {
            array_element_types.insert(*base, *result);
        }
    }

    // Apply defaults and track which type IDs were defaulted
    let mut defaulted_ty_ids: std::collections::HashSet<TyId> = std::collections::HashSet::new();
    let mut any_applied = false;

    for (ty_id, feature, span) in literals_to_default {
        let default_ty = match feature {
            LanguageFeature::ExpressibleByIntLiteral => {
                ctx.oracle().default_integer_type(span.clone())
            },
            LanguageFeature::ExpressibleByFloatLiteral => {
                ctx.oracle().default_float_type(span.clone())
            },
            LanguageFeature::ExpressibleByStringLiteral => {
                ctx.oracle().default_string_type(span.clone())
            },
            LanguageFeature::ExpressibleByBoolLiteral => {
                ctx.oracle().default_boolean_type(span.clone())
            },
            LanguageFeature::ExpressibleByCharLiteral => {
                ctx.oracle().default_char_type(span.clone())
            },
            // Null literal default is generic (NullLiteralType[T] = Optional[T]),
            // so we can't apply a concrete default - type must be inferred from context
            LanguageFeature::ExpressibleByNullLiteral => continue,
            // Array literal default is Array[ElementType]
            LanguageFeature::_ExpressibleByArrayLiteral => {
                // Find the element type from the Normalizes constraint
                if let Some(&elem_ty_id) = array_element_types.get(&ty_id) {
                    let elem_ty = resolve_type(ctx, elem_ty_id);
                    // If element type is still Infer, skip for now - inner elements
                    // need to get defaults first (for nested arrays like [[1,2],[3,4]])
                    if matches!(elem_ty.kind(), TyKind::Infer) {
                        continue;
                    }
                    if let Some(array_ty) = ctx.oracle().default_array_type(elem_ty, span.clone()) {
                        array_ty
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            },
            _ => continue,
        };

        // Register the default type and add an equality constraint.
        // Using equate() instead of direct substitution ensures the default
        // propagates through any existing unification chains (e.g., when an
        // array literal is inside a tuple, the array's type may be equated
        // with the tuple's element type slot).
        ctx.register_type(&default_ty);
        ctx.equate(ty_id, default_ty.id(), span);
        defaulted_ty_ids.insert(ty_id);
        any_applied = true;
    }

    // Put constraints back (except for literal constraints that we actually defaulted)
    for constraint in constraints {
        if let Constraint::Conforms { ty, protocol } = &constraint {
            // Only skip constraints for types we actually defaulted
            if defaulted_ty_ids.contains(ty)
                && get_literal_feature_for_protocol(ctx, protocol.symbol_id).is_some()
            {
                // Skip this constraint - we applied a default type
                continue;
            }
        }
        ctx.push_constraint(constraint);
    }

    any_applied
}

/// Check if a protocol symbol ID corresponds to a literal expression protocol.
fn get_literal_feature_for_protocol(
    ctx: &InferenceContext<'_>,
    protocol_id: semantic_tree::symbol::SymbolId,
) -> Option<LanguageFeature> {
    // Check each literal protocol
    let features = [
        LanguageFeature::ExpressibleByIntLiteral,
        LanguageFeature::ExpressibleByFloatLiteral,
        LanguageFeature::ExpressibleByStringLiteral,
        LanguageFeature::ExpressibleByBoolLiteral,
        LanguageFeature::ExpressibleByCharLiteral,
        LanguageFeature::ExpressibleByNullLiteral,
        LanguageFeature::_ExpressibleByArrayLiteral,
        LanguageFeature::_ExpressibleByDictionaryLiteral,
    ];

    for feature in features {
        if let Some(id) = ctx.oracle().builtin_protocol(feature)
            && id == protocol_id
        {
            return Some(feature);
        }
    }

    None
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
            },
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
            arguments,
            result,
            expr_id,
            span,
        } => resolve_member(
            ctx, *receiver, member, *is_static, arguments, *result, *expr_id, span,
        ),
        Constraint::ImplicitMember {
            expr_ty,
            member_name,
            argument_tys,
            expr_id,
            span,
        } => resolve_implicit_member(ctx, *expr_ty, member_name, argument_tys, *expr_id, span),
        Constraint::EnumPatternBinding {
            enum_ty,
            case_name,
            binding_tys,
            span,
        } => resolve_enum_pattern_binding(ctx, *enum_ty, case_name, binding_tys, span),
        Constraint::StructPatternBinding {
            struct_ty,
            struct_name,
            field_bindings,
            has_rest,
            span,
        } => resolve_struct_pattern_binding(
            ctx,
            *struct_ty,
            struct_name,
            field_bindings,
            *has_rest,
            span,
        ),
        Constraint::Promotable {
            from_ty,
            to_ty,
            expr_id,
            span,
        } => resolve_promotable(ctx, *from_ty, *to_ty, *expr_id, span),
        Constraint::TupleIndexAccess {
            tuple,
            index,
            result,
            span,
        } => resolve_tuple_index(ctx, *tuple, *index, *result, span),
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

    // Normalize type parameters/associated types using equality constraints from where clauses.
    // This helps unify types like `I.Iter.Item` with `T` when constraints equate them.
    let norm_a = normalize_with_constraints_if_needed(ctx, &ty_a);
    let norm_b = normalize_with_constraints_if_needed(ctx, &ty_b);
    if norm_a.to_string() != ty_a.to_string() || norm_b.to_string() != ty_b.to_string() {
        return unify(ctx, norm_a.id(), norm_b.id(), span);
    }

    // Keep track of original IDs for closure metadata lookup
    let original_a = a;
    let original_b = b;

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
        },

        // Error types unify with anything (suppress cascading errors)
        (TyKind::Error, _) | (_, TyKind::Error) => Ok(SolveResult::Solved),

        // Never (bottom type) unifies with anything.
        // IMPORTANT: Never should NOT substitute an Infer type, because Never is the
        // bottom type that can coerce to any other type. If we have:
        //   match x { true => 42, false => return }
        // The match type should be Int (from the first arm), not Never.
        // When Infer is unified with Never, we succeed without substituting,
        // allowing another constraint to give Infer a concrete type.
        (TyKind::Never, TyKind::Infer) | (TyKind::Infer, TyKind::Never) => {
            // Don't substitute - leave the Infer type open for other constraints
            Ok(SolveResult::Solved)
        },
        (TyKind::Never, _) | (_, TyKind::Never) => Ok(SolveResult::Solved),

        // One is an inference placeholder - substitute it
        (TyKind::Infer, _) => {
            if occurs_check(ty_a.id(), &ty_b, ctx) {
                return Err(InferenceError::occurs_check(
                    ty_a.id(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }
            ctx.substitutions_mut().insert(ty_a.id(), ty_b.clone());
            Ok(SolveResult::Solved)
        },
        (_, TyKind::Infer) => {
            if occurs_check(ty_b.id(), &ty_a, ctx) {
                return Err(InferenceError::occurs_check(
                    ty_b.id(),
                    ty_a.clone(),
                    span.clone(),
                ));
            }
            ctx.substitutions_mut().insert(ty_b.id(), ty_a.clone());
            Ok(SolveResult::Solved)
        },

        // UnresolvedFunction with Function - check param info compatibility
        (
            TyKind::UnresolvedFunction {
                param_info,
                return_type: ret_unresolved,
            },
            TyKind::Function {
                params: expected_params,
                return_type: expected_return,
            },
        ) => {
            match param_info {
                ParamInfo::Unconstrained => {
                    // Accept any arity - no param constraints to check
                    // Just unify return types
                },
                ParamInfo::ImplicitIt { it_type } => {
                    if expected_params.len() != 1 {
                        return Err(InferenceError::it_used_with_wrong_arity(
                            expected_params.len(),
                            span.clone(),
                        ));
                    }
                    // Equate the it_type with the expected single parameter
                    ctx.equate(it_type.id(), expected_params[0].id(), span.clone());
                },
                ParamInfo::Explicit { param_types } => {
                    if param_types.len() != expected_params.len() {
                        return Err(InferenceError::closure_arity_mismatch(
                            param_types.len(),
                            expected_params.len(),
                            span.clone(),
                        ));
                    }
                    for (a, b) in param_types.iter().zip(expected_params.iter()) {
                        ctx.equate(a.id(), b.id(), span.clone());
                    }
                },
            }
            // Unify return types
            ctx.equate(ret_unresolved.id(), expected_return.id(), span.clone());
            // Store substitution: UnresolvedFunction -> Function
            ctx.substitutions_mut().insert(ty_a.id(), ty_b.clone());
            Ok(SolveResult::Solved)
        },

        // Function with UnresolvedFunction (symmetric case)
        (
            TyKind::Function {
                params: expected_params,
                return_type: expected_return,
            },
            TyKind::UnresolvedFunction {
                param_info,
                return_type: ret_unresolved,
            },
        ) => {
            match param_info {
                ParamInfo::Unconstrained => {
                    // Accept any arity - no param constraints to check
                },
                ParamInfo::ImplicitIt { it_type } => {
                    if expected_params.len() != 1 {
                        return Err(InferenceError::it_used_with_wrong_arity(
                            expected_params.len(),
                            span.clone(),
                        ));
                    }
                    ctx.equate(it_type.id(), expected_params[0].id(), span.clone());
                },
                ParamInfo::Explicit { param_types } => {
                    if param_types.len() != expected_params.len() {
                        return Err(InferenceError::closure_arity_mismatch(
                            param_types.len(),
                            expected_params.len(),
                            span.clone(),
                        ));
                    }
                    for (a, b) in param_types.iter().zip(expected_params.iter()) {
                        ctx.equate(a.id(), b.id(), span.clone());
                    }
                },
            }
            ctx.equate(ret_unresolved.id(), expected_return.id(), span.clone());
            // Store substitution: UnresolvedFunction -> Function
            ctx.substitutions_mut().insert(ty_b.id(), ty_a.clone());
            Ok(SolveResult::Solved)
        },

        // UnresolvedFunction with UnresolvedFunction
        (
            TyKind::UnresolvedFunction {
                param_info: info_a,
                return_type: ret_a,
            },
            TyKind::UnresolvedFunction {
                param_info: info_b,
                return_type: ret_b,
            },
        ) => {
            // Unify param infos if compatible
            match (info_a, info_b) {
                (ParamInfo::Unconstrained, _) | (_, ParamInfo::Unconstrained) => {
                    // One is unconstrained, the other's constraints win
                    // Just unify return types
                },
                (
                    ParamInfo::ImplicitIt { it_type: it_a },
                    ParamInfo::ImplicitIt { it_type: it_b },
                ) => {
                    // Both use it - unify the it types
                    ctx.equate(it_a.id(), it_b.id(), span.clone());
                },
                (
                    ParamInfo::Explicit {
                        param_types: params_a,
                    },
                    ParamInfo::Explicit {
                        param_types: params_b,
                    },
                ) => {
                    // Both have explicit params - must match arity
                    if params_a.len() != params_b.len() {
                        return Err(InferenceError::closure_arity_mismatch(
                            params_a.len(),
                            params_b.len(),
                            span.clone(),
                        ));
                    }
                    for (a, b) in params_a.iter().zip(params_b.iter()) {
                        ctx.equate(a.id(), b.id(), span.clone());
                    }
                },
                (ParamInfo::ImplicitIt { it_type }, ParamInfo::Explicit { param_types })
                | (ParamInfo::Explicit { param_types }, ParamInfo::ImplicitIt { it_type }) => {
                    // ImplicitIt requires exactly 1 param
                    if param_types.len() != 1 {
                        return Err(InferenceError::closure_arity_mismatch(
                            1,
                            param_types.len(),
                            span.clone(),
                        ));
                    }
                    ctx.equate(it_type.id(), param_types[0].id(), span.clone());
                },
            }
            ctx.equate(ret_a.id(), ret_b.id(), span.clone());
            Ok(SolveResult::Solved)
        },

        // Structural unification for compound types
        (TyKind::Tuple(elems_a), TyKind::Tuple(elems_b)) => {
            if elems_a.len() != elems_b.len() {
                return Err(InferenceError::tuple_arity_mismatch(
                    elems_b.len(), // expected (the scrutinee type)
                    elems_a.len(), // found (the pattern)
                    span.clone(),
                ));
            }
            for (ea, eb) in elems_a.iter().zip(elems_b.iter()) {
                ctx.equate(ea.id(), eb.id(), span.clone());
            }
            Ok(SolveResult::Solved)
        },

        // Note: Array[T] struct types are unified via the Struct case above
        (TyKind::Pointer(elem_a), TyKind::Pointer(elem_b)) => {
            ctx.equate(elem_a.id(), elem_b.id(), span.clone());
            Ok(SolveResult::Solved)
        },

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
            // Check if either side is a closure - if so, emit closure-specific errors
            // Check both the resolved type ID and the original ID
            let closure_a = ctx
                .closure_metadata()
                .get(&ty_a.id())
                .cloned()
                .or_else(|| ctx.closure_metadata().get(&original_a).cloned());
            let closure_b = ctx
                .closure_metadata()
                .get(&ty_b.id())
                .cloned()
                .or_else(|| ctx.closure_metadata().get(&original_b).cloned());

            // Determine which type is the closure and which is the expected type
            if let Some(closure_meta) = closure_a {
                // ty_a is a closure, ty_b is expected type
                let expected_params = params_b;
                let expected_return = ret_b;

                // 1. Check if `it` is used with wrong arity
                if closure_meta.uses_it && expected_params.len() != 1 {
                    return Err(InferenceError::it_used_with_wrong_arity(
                        expected_params.len(),
                        closure_meta.span.clone(),
                    ));
                }

                // 2. Check arity mismatch
                // For implicit-it closures (param_count=0 and no explicit params), arity is determined by uses_it
                let expected_arity =
                    if closure_meta.param_count == 0 && !closure_meta.has_explicit_params {
                        // Implicit-it closure: arity is 1 if uses_it, else 0
                        if closure_meta.uses_it { 1 } else { 0 }
                    } else {
                        // Explicit-param closure: use param_count directly
                        closure_meta.param_count
                    };

                if expected_arity != expected_params.len() {
                    return Err(InferenceError::closure_arity_mismatch(
                        expected_arity,
                        expected_params.len(),
                        closure_meta.span.clone(),
                    ));
                }

                // 3. Check parameter types (generate constraints for now, errors will surface later)
                for (pa, pb) in params_a.iter().zip(expected_params.iter()) {
                    ctx.equate(pa.id(), pb.id(), span.clone());
                }

                // 4. Check return type (generate constraint for now, errors will surface later)
                ctx.equate(ret_a.id(), expected_return.id(), span.clone());

                Ok(SolveResult::Solved)
            } else if let Some(closure_meta) = closure_b {
                // ty_b is a closure, ty_a is expected type
                let expected_params = params_a;
                let expected_return = ret_a;

                // 1. Check if `it` is used with wrong arity
                if closure_meta.uses_it && expected_params.len() != 1 {
                    return Err(InferenceError::it_used_with_wrong_arity(
                        expected_params.len(),
                        closure_meta.span.clone(),
                    ));
                }

                // 2. Check arity mismatch
                // For implicit-it closures (param_count=0 and no explicit params), arity is determined by uses_it
                let expected_arity =
                    if closure_meta.param_count == 0 && !closure_meta.has_explicit_params {
                        // Implicit-it closure: arity is 1 if uses_it, else 0
                        if closure_meta.uses_it { 1 } else { 0 }
                    } else {
                        // Explicit-param closure: use param_count directly
                        closure_meta.param_count
                    };

                if expected_arity != expected_params.len() {
                    return Err(InferenceError::closure_arity_mismatch(
                        expected_arity,
                        expected_params.len(),
                        closure_meta.span.clone(),
                    ));
                }

                // 3. Check parameter types (generate constraints for now, errors will surface later)
                for (pa, pb) in params_b.iter().zip(expected_params.iter()) {
                    ctx.equate(pa.id(), pb.id(), span.clone());
                }

                // 4. Check return type (generate constraint for now, errors will surface later)
                ctx.equate(ret_b.id(), expected_return.id(), span.clone());

                Ok(SolveResult::Solved)
            } else {
                // Neither is a closure - use generic function mismatch error
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
        },

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
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

            if id_a != id_b {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }

            // Unify substitutions by matching keys (type parameter IDs)
            // HashMap iteration order is non-deterministic, so we must match by key
            for (key, sub_a) in subs_a.iter() {
                if let Some(sub_b) = subs_b.get(*key) {
                    ctx.equate(sub_a.id(), sub_b.id(), span.clone());
                } else {
                    // Substitution key missing in other type - structural mismatch
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            // Check for keys in b that aren't in a
            for (key, _) in subs_b.iter() {
                if !subs_a.contains(*key) {
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            Ok(SolveResult::Solved)
        },

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
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

            if id_a != id_b {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }

            // Unify substitutions by matching keys (type parameter IDs)
            for (key, sub_a) in subs_a.iter() {
                if let Some(sub_b) = subs_b.get(*key) {
                    ctx.equate(sub_a.id(), sub_b.id(), span.clone());
                } else {
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            for (key, _) in subs_b.iter() {
                if !subs_a.contains(*key) {
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            Ok(SolveResult::Solved)
        },

        (
            TyKind::Enum {
                symbol: sym_a,
                substitutions: subs_a,
            },
            TyKind::Enum {
                symbol: sym_b,
                substitutions: subs_b,
            },
        ) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();

            if id_a != id_b {
                return Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ));
            }

            // Unify substitutions by matching keys (type parameter IDs)
            for (key, sub_a) in subs_a.iter() {
                if let Some(sub_b) = subs_b.get(*key) {
                    ctx.equate(sub_a.id(), sub_b.id(), span.clone());
                } else {
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            for (key, _) in subs_b.iter() {
                if !subs_a.contains(*key) {
                    return Err(InferenceError::type_mismatch(
                        ty_a.clone(),
                        ty_b.clone(),
                        span.clone(),
                    ));
                }
            }
            Ok(SolveResult::Solved)
        },

        // Type parameters - only equal if they're the same parameter
        (TyKind::TypeParameter(param_a), TyKind::TypeParameter(param_b)) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

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
        },

        // Associated types - defer if not yet resolved
        (TyKind::AssociatedType { .. }, _) | (_, TyKind::AssociatedType { .. }) => {
            // Associated types need to be normalized first
            Ok(SolveResult::Deferred)
        },

        // Self type matches Self or compatible struct/protocol
        (TyKind::SelfType, TyKind::SelfType) => Ok(SolveResult::Solved),
        (TyKind::SelfType, TyKind::Struct { .. }) | (TyKind::Struct { .. }, TyKind::SelfType) => {
            Ok(SolveResult::Solved)
        },
        (TyKind::SelfType, TyKind::Protocol { .. })
        | (TyKind::Protocol { .. }, TyKind::SelfType) => Ok(SolveResult::Solved),

        // Struct to Protocol - check conformance
        (TyKind::Struct { substitutions, .. }, TyKind::Protocol { symbol, .. }) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            // Defer if struct has unresolved inference placeholders in substitutions
            // (conformance check needs fully resolved types to match extensions)
            if substitutions
                .iter()
                .any(|(_, ty)| matches!(ty.kind(), TyKind::Infer))
            {
                return Ok(SolveResult::Deferred);
            }

            let protocol_id = Symbol::<KestrelLanguage>::metadata(symbol.as_ref()).id();
            if ctx.oracle().conforms_to(&ty_a, protocol_id) {
                Ok(SolveResult::Solved)
            } else {
                Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ))
            }
        },
        (TyKind::Protocol { symbol, .. }, TyKind::Struct { substitutions, .. }) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            // Defer if struct has unresolved inference placeholders in substitutions
            if substitutions
                .iter()
                .any(|(_, ty)| matches!(ty.kind(), TyKind::Infer))
            {
                return Ok(SolveResult::Deferred);
            }

            let protocol_id = Symbol::<KestrelLanguage>::metadata(symbol.as_ref()).id();
            if ctx.oracle().conforms_to(&ty_b, protocol_id) {
                Ok(SolveResult::Solved)
            } else {
                Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ))
            }
        },

        // Enum to Protocol - check conformance
        (TyKind::Enum { substitutions, .. }, TyKind::Protocol { symbol, .. }) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            // Defer if enum has unresolved inference placeholders in substitutions
            if substitutions
                .iter()
                .any(|(_, ty)| matches!(ty.kind(), TyKind::Infer))
            {
                return Ok(SolveResult::Deferred);
            }

            let protocol_id = Symbol::<KestrelLanguage>::metadata(symbol.as_ref()).id();
            if ctx.oracle().conforms_to(&ty_a, protocol_id) {
                Ok(SolveResult::Solved)
            } else {
                Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ))
            }
        },
        (TyKind::Protocol { symbol, .. }, TyKind::Enum { substitutions, .. }) => {
            use kestrel_semantic_tree::language::KestrelLanguage;
            use semantic_tree::symbol::Symbol;

            // Defer if enum has unresolved inference placeholders in substitutions
            if substitutions
                .iter()
                .any(|(_, ty)| matches!(ty.kind(), TyKind::Infer))
            {
                return Ok(SolveResult::Deferred);
            }

            let protocol_id = Symbol::<KestrelLanguage>::metadata(symbol.as_ref()).id();
            if ctx.oracle().conforms_to(&ty_b, protocol_id) {
                Ok(SolveResult::Solved)
            } else {
                Err(InferenceError::type_mismatch(
                    ty_a.clone(),
                    ty_b.clone(),
                    span.clone(),
                ))
            }
        },

        // Primitive types - exact match required
        (TyKind::Unit, TyKind::Unit) => Ok(SolveResult::Solved),
        (TyKind::Bool, TyKind::Bool) => Ok(SolveResult::Solved),
        (TyKind::String, TyKind::String) => Ok(SolveResult::Solved),
        (TyKind::Int(bits_a), TyKind::Int(bits_b)) if bits_a == bits_b => Ok(SolveResult::Solved),
        (TyKind::Float(bits_a), TyKind::Float(bits_b)) if bits_a == bits_b => {
            Ok(SolveResult::Solved)
        },

        // Type aliases - expand and retry
        (TyKind::TypeAlias { .. }, _) => {
            let expanded = ctx.oracle().expand_type_alias(&ty_a);
            // Register the expanded type so resolve_type can find it
            ctx.register_type(&expanded);
            ctx.equate(expanded.id(), ty_b.id(), span.clone());
            Ok(SolveResult::Solved)
        },
        (_, TyKind::TypeAlias { .. }) => {
            let expanded = ctx.oracle().expand_type_alias(&ty_b);
            // Register the expanded type so resolve_type can find it
            ctx.register_type(&expanded);
            ctx.equate(ty_a.id(), expanded.id(), span.clone());
            Ok(SolveResult::Solved)
        },

        // No match - type mismatch
        _ => Err(InferenceError::type_mismatch(
            ty_a.clone(),
            ty_b.clone(),
            span.clone(),
        )),
    }
}

/// Resolve a promotable constraint.
///
/// A promotable constraint first tries unification. If that fails, it checks if
/// the target type conforms to `FromValue[source]` and records a promotion if so.
/// This enables implicit wrapping of values in Optional or Result types.
fn resolve_promotable(
    ctx: &mut InferenceContext<'_>,
    from_ty: TyId,
    to_ty: TyId,
    expr_id: kestrel_semantic_tree::expr::ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let from = resolve_type(ctx, from_ty);
    let to = resolve_type(ctx, to_ty);

    // Expand type aliases for both types to get the underlying types
    let from = from.expand_aliases();
    let to = to.expand_aliases();

    // If both types have unresolved variables, unify to link them.
    if contains_unresolved_infer(&from) && contains_unresolved_infer(&to) {
        return unify(ctx, from_ty, to_ty, span);
    }

    // If the source type is a pure Infer placeholder, we need to decide:
    // - If it's a literal AND target is Infer or a promotion target (Optional/Result),
    //   defer so the literal type resolves first, then we can try promotion.
    //   Example: `let x: Int? = 5` - the literal 5 should resolve to Int64 first.
    //   Example: When target is Infer, it might become Optional later via type annotation.
    // - Otherwise, unify to propagate type context from the target.
    //   Example: `return .Ok(value)` needs the Result type from the return type.
    //   Example: `Int64(intLiteral: 0)` - the literal 0 should take type I64 from context.
    if matches!(from.kind(), TyKind::Infer) {
        if ctx.is_literal_ty(from_ty) {
            // For literals, defer if target is Infer (might become Optional later)
            // or if target is a known promotion target
            if matches!(to.kind(), TyKind::Infer) || is_potential_promotion_target(&to) {
                return Ok(SolveResult::Deferred);
            }
            // Target is concrete and not a promotion target - unify to propagate context
            return unify(ctx, from_ty, to_ty, span);
        } else {
            // Not a literal - unify to propagate type context
            return unify(ctx, from_ty, to_ty, span);
        }
    }

    // If the target type has unresolved variables (but source is resolved), unify.
    if contains_unresolved_infer(&to) {
        return unify(ctx, from_ty, to_ty, span);
    }

    // Normalize type parameters/associated types using equality constraints before
    // deciding whether the types can unify or should be promoted.
    let from_normalized = normalize_with_constraints_if_needed(ctx, &from);
    let to_normalized = normalize_with_constraints_if_needed(ctx, &to);

    // Check if types are potentially unifiable (same kind or compatible kinds).
    // If so, try to unify. If not, check for FromValue promotion.
    if types_could_unify(&from_normalized, &to_normalized) {
        // Types could be unifiable - register the expanded types and try unify
        ctx.register_type(&from_normalized);
        ctx.register_type(&to_normalized);
        return unify(ctx, from_normalized.id(), to_normalized.id(), span);
    }

    // Types can't unify (different kinds). Check if target conforms to FromValue[source] for promotion.
    if let Some((method_id, subs)) =
        ctx.oracle()
            .check_from_value_conformance(&to_normalized, &from_normalized)
    {
        // Record promotion for apply_solution
        ctx.promotions_mut()
            .insert(expr_id, PromotionInfo::new(to_normalized.clone(), method_id, subs));
        return Ok(SolveResult::Solved);
    }

    // Neither direct unification nor promotion worked - report type mismatch
    Err(InferenceError::type_mismatch(
        to_normalized,
        from_normalized,
        span.clone(),
    ))
}

/// Normalize type parameters and associated types using equality constraints from context.
fn normalize_with_constraints_if_needed(ctx: &mut InferenceContext<'_>, ty: &Ty) -> Ty {
    if matches!(
        ty.kind(),
        TyKind::TypeParameter { .. } | TyKind::AssociatedType { .. }
    ) {
        let normalized = ctx.oracle().normalize_with_constraints(ty);
        if normalized.to_string() != ty.to_string() {
            ctx.register_type(&normalized);
            return normalized;
        }
    }
    ty.clone()
}

/// Check if a type is a potential promotion target (Optional or Result).
///
/// These are the types that implement FromValue and thus could accept promoted values.
/// For literals, we only defer when the target is one of these types.
fn is_potential_promotion_target(ty: &Ty) -> bool {
    use kestrel_semantic_tree::language::KestrelLanguage;
    use semantic_tree::symbol::Symbol;

    let ty = ty.expand_aliases();
    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            let name = &Symbol::<KestrelLanguage>::metadata(symbol.as_ref())
                .name()
                .value;
            name == "Optional" || name == "Result"
        },
        TyKind::Enum { symbol, .. } => {
            let name = &Symbol::<KestrelLanguage>::metadata(symbol.as_ref())
                .name()
                .value;
            name == "Optional" || name == "Result"
        },
        // Handle type aliases that might not be expanded yet
        TyKind::TypeAlias { symbol, .. } => {
            let name = &Symbol::<KestrelLanguage>::metadata(symbol.as_ref())
                .name()
                .value;
            name == "OptionalTypeOperator" || name == "ResultTypeOperator"
        },
        _ => false,
    }
}

/// Check if a type contains any unresolved inference variables.
fn contains_unresolved_infer(ty: &Ty) -> bool {
    match ty.kind() {
        TyKind::Infer => true,
        TyKind::Tuple(elements) => elements.iter().any(contains_unresolved_infer),
        TyKind::Pointer(elem) => contains_unresolved_infer(elem),
        TyKind::Function {
            params,
            return_type,
        } => params.iter().any(contains_unresolved_infer) || contains_unresolved_infer(return_type),
        TyKind::Struct { substitutions, .. }
        | TyKind::Enum { substitutions, .. }
        | TyKind::Protocol { substitutions, .. } => substitutions
            .iter()
            .any(|(_, t)| contains_unresolved_infer(t)),
        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            if contains_unresolved_infer(return_type) {
                return true;
            }
            match param_info {
                ParamInfo::ImplicitIt { it_type } => contains_unresolved_infer(it_type),
                ParamInfo::Explicit { param_types } => {
                    param_types.iter().any(contains_unresolved_infer)
                },
                ParamInfo::Unconstrained => false,
            }
        },
        TyKind::AssociatedType { container, .. } => container
            .as_ref()
            .is_some_and(|c| contains_unresolved_infer(c)),
        _ => false,
    }
}

/// Check if two types could potentially unify (are of compatible kinds).
/// This is used by Promotable to decide whether to try unification or promotion.
fn types_could_unify(a: &Ty, b: &Ty) -> bool {
    use kestrel_semantic_tree::language::KestrelLanguage;
    use semantic_tree::symbol::Symbol;

    // Handle special types that unify with anything
    if matches!(a.kind(), TyKind::Infer | TyKind::Error | TyKind::Never) {
        return true;
    }
    if matches!(b.kind(), TyKind::Infer | TyKind::Error | TyKind::Never) {
        return true;
    }

    // Check if types are of compatible kinds
    match (a.kind(), b.kind()) {
        // Same kind with same symbol - can unify
        (TyKind::Struct { symbol: sym_a, .. }, TyKind::Struct { symbol: sym_b, .. }) => {
            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();
            id_a == id_b
        },
        (TyKind::Enum { symbol: sym_a, .. }, TyKind::Enum { symbol: sym_b, .. }) => {
            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();
            id_a == id_b
        },
        (TyKind::Protocol { symbol: sym_a, .. }, TyKind::Protocol { symbol: sym_b, .. }) => {
            let id_a = Symbol::<KestrelLanguage>::metadata(sym_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(sym_b.as_ref()).id();
            id_a == id_b
        },
        // Struct/Enum to Protocol - might conform
        (TyKind::Struct { .. }, TyKind::Protocol { .. }) => true,
        (TyKind::Enum { .. }, TyKind::Protocol { .. }) => true,
        (TyKind::Protocol { .. }, TyKind::Struct { .. }) => true,
        (TyKind::Protocol { .. }, TyKind::Enum { .. }) => true,
        // Primitive types
        (TyKind::Unit, TyKind::Unit) => true,
        (TyKind::Bool, TyKind::Bool) => true,
        (TyKind::String, TyKind::String) => true,
        (TyKind::Int(bits_a), TyKind::Int(bits_b)) => bits_a == bits_b,
        (TyKind::Float(bits_a), TyKind::Float(bits_b)) => bits_a == bits_b,
        // Structural types
        (TyKind::Tuple(_), TyKind::Tuple(_)) => true,
        (TyKind::Pointer(_), TyKind::Pointer(_)) => true,
        (TyKind::Function { .. }, TyKind::Function { .. }) => true,
        (TyKind::UnresolvedFunction { .. }, TyKind::Function { .. }) => true,
        (TyKind::Function { .. }, TyKind::UnresolvedFunction { .. }) => true,
        (TyKind::UnresolvedFunction { .. }, TyKind::UnresolvedFunction { .. }) => true,
        // Type parameters
        (TyKind::TypeParameter(param_a), TyKind::TypeParameter(param_b)) => {
            let id_a = Symbol::<KestrelLanguage>::metadata(param_a.as_ref()).id();
            let id_b = Symbol::<KestrelLanguage>::metadata(param_b.as_ref()).id();
            id_a == id_b
        },
        // Self type
        (TyKind::SelfType, TyKind::SelfType) => true,
        (TyKind::SelfType, TyKind::Struct { .. }) => true,
        (TyKind::Struct { .. }, TyKind::SelfType) => true,
        (TyKind::SelfType, TyKind::Protocol { .. }) => true,
        (TyKind::Protocol { .. }, TyKind::SelfType) => true,
        // Associated types need deferred resolution
        (TyKind::AssociatedType { .. }, _) => true,
        (_, TyKind::AssociatedType { .. }) => true,
        // Type aliases should have been expanded - if we get here, treat as potentially unifiable
        (TyKind::TypeAlias { .. }, _) => true,
        (_, TyKind::TypeAlias { .. }) => true,
        // Different kinds - can't unify directly
        _ => false,
    }
}

/// Check if a type conforms to a protocol.
fn check_conforms(
    ctx: &mut InferenceContext<'_>,
    ty: TyId,
    protocol: &crate::constraint::ProtocolRef,
) -> Result<SolveResult, InferenceError> {
    let mut resolved = resolve_type(ctx, ty);

    // If the type is still an inference placeholder, defer
    if matches!(resolved.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Expand type aliases before conformance checking.
    // Type aliases like DictionaryTypeOperator -> Dictionary need to be expanded
    // so we can check conformance on the actual underlying type.
    while matches!(resolved.kind(), TyKind::TypeAlias { .. }) {
        resolved = ctx.oracle().expand_type_alias(&resolved);
        ctx.register_type(&resolved);
    }

    // Check conformance via the oracle
    if ctx.oracle().conforms_to(&resolved, protocol.symbol_id) {
        Ok(SolveResult::Solved)
    } else {
        let protocol_name = ctx
            .oracle()
            .symbol_name(protocol.symbol_id)
            .unwrap_or_else(|| format!("{:?}", protocol.symbol_id));
        Err(InferenceError::conformance_failure(
            resolved.clone(),
            protocol_name,
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
    let mut base_ty = resolve_type(ctx, base);

    // Expand type aliases before associated type lookup.
    // Type aliases (e.g., ArrayTypeOperator -> Array) need to be expanded to their
    // underlying type so we can look up associated types on the actual struct.
    while matches!(base_ty.kind(), TyKind::TypeAlias { .. }) {
        base_ty = ctx.oracle().expand_type_alias(&base_ty);
        ctx.register_type(&base_ty);
    }

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
        },
        None => Err(InferenceError::associated_type_not_found(
            base_ty.clone(),
            assoc_name.to_string(),
            span.clone(),
        )),
    }
}

/// Resolve a tuple index access.
///
/// This is called when tuple indexing is deferred because the tuple type
/// wasn't known at constraint generation time (e.g., type parameter with
/// a tuple constraint like `where Item = (A, B)`).
fn resolve_tuple_index(
    ctx: &mut InferenceContext<'_>,
    tuple: TyId,
    index: usize,
    result: TyId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let mut tuple_ty = resolve_type(ctx, tuple);

    // Expand type aliases before tuple access
    while matches!(tuple_ty.kind(), TyKind::TypeAlias { .. }) {
        tuple_ty = ctx.oracle().expand_type_alias(&tuple_ty);
        ctx.register_type(&tuple_ty);
    }

    // Normalize type parameters and associated types using equality constraints.
    // This handles cases like `Item = (K, V)` and `I.Item = (K, V)` where tuple
    // indexing should work on values of the constrained type.
    tuple_ty = normalize_with_constraints_if_needed(ctx, &tuple_ty);

    // If the tuple type is still an inference placeholder, defer
    if matches!(tuple_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Check if it's a tuple
    match tuple_ty.as_tuple() {
        Some(elements) => {
            if index >= elements.len() {
                return Err(InferenceError::tuple_index_out_of_bounds(
                    index,
                    elements.len(),
                    span.clone(),
                ));
            }
            let element_ty = elements[index].clone();
            ctx.register_type(&element_ty);
            ctx.equate(element_ty.id(), result, span.clone());
            Ok(SolveResult::Solved)
        },
        None => Err(InferenceError::tuple_index_on_non_tuple(
            tuple_ty.clone(),
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
    arguments: &[TyId],
    result: TyId,
    expr_id: kestrel_semantic_tree::expr::ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    let mut receiver_ty = resolve_type(ctx, receiver);

    // Expand type aliases before member lookup.
    // Type aliases (e.g., Int -> Int64) need to be expanded to their underlying
    // type so we can look up methods on the actual struct.
    while matches!(receiver_ty.kind(), TyKind::TypeAlias { .. }) {
        receiver_ty = ctx.oracle().expand_type_alias(&receiver_ty);
        ctx.register_type(&receiver_ty);
    }

    // Normalize type parameters and associated types using equality constraints from where clauses.
    // This handles cases like `where V = Array[E]` where methods should be callable on type
    // parameter V by substituting it with Array[E].
    if matches!(
        receiver_ty.kind(),
        TyKind::TypeParameter { .. } | TyKind::AssociatedType { .. }
    ) {
        let normalized = ctx.oracle().normalize_with_constraints(&receiver_ty);
        if normalized.to_string() != receiver_ty.to_string() {
            receiver_ty = normalized;
            ctx.register_type(&receiver_ty);
        }
    }

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

            // Create constraints for argument types vs parameter types.
            // This enables proper type inference for literals in expressions like `int32 + 5`
            // where the literal `5` should be constrained to Int32 (not defaulted to Int64).
            for (arg_ty_id, param_ty) in arguments.iter().zip(resolution.parameters.iter()) {
                ctx.register_type(param_ty);
                ctx.equate(*arg_ty_id, param_ty.id(), span.clone());
            }

            // For Self-returning methods (like negate()), equate receiver with result.
            // This enables bidirectional type inference: when the result type is constrained
            // by context (e.g., Int16), that constraint propagates back to the receiver,
            // allowing literals like `-32768` to infer correctly as Int16.
            if resolution.returns_self {
                ctx.equate(receiver, result, span.clone());
            }

            Ok(SolveResult::Solved)
        },
        Err(MemberError::UnknownType) => {
            // Shouldn't happen since we checked for Infer above, but defer anyway
            Ok(SolveResult::Deferred)
        },
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

/// Resolve an implicit member access (enum shorthand like `.SomeCase`).
fn resolve_implicit_member(
    ctx: &mut InferenceContext<'_>,
    expr_ty: TyId,
    member_name: &str,
    argument_tys: &[(Option<String>, TyId)],
    expr_id: kestrel_semantic_tree::expr::ExprId,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    #[allow(unused_imports)]
    use kestrel_semantic_tree::behavior::callable::CallableBehavior;
    #[allow(unused_imports)]
    use kestrel_semantic_tree::symbol::enum_case::EnumCaseSymbol;
    use semantic_tree::symbol::Symbol;

    let resolved_ty = resolve_type(ctx, expr_ty);

    // Expand type aliases to get underlying type (e.g., OptionalTypeOperator -> Optional)
    let resolved_ty = resolved_ty.expand_aliases();

    // If still Infer, defer until expected type is known
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // If still a TypeAlias after expansion, the alias hasn't been bound yet.
    // This happens during bootstrap (e.g., result.ks using T throws E before
    // ResultTypeOperator is fully bound). Defer to allow binding to complete.
    if matches!(resolved_ty.kind(), TyKind::TypeAlias { .. }) {
        return Ok(SolveResult::Deferred);
    }

    // Must be an enum type
    let TyKind::Enum {
        symbol: enum_symbol,
        substitutions,
    } = resolved_ty.kind()
    else {
        // Not an enum - error
        return Err(InferenceError::member_not_found(
            resolved_ty.clone(),
            member_name.to_string(),
            span.clone(),
        ));
    };

    // Find the case by name
    let cases = enum_symbol.cases();
    let case = cases
        .iter()
        .find(|c| c.metadata().name().value == member_name);

    let Some(case) = case else {
        return Err(InferenceError::member_not_found(
            resolved_ty.clone(),
            member_name.to_string(),
            span.clone(),
        ));
    };

    // Check if case has associated values (CallableBehavior)
    let callable = case.callable_behavior();

    match (&callable, argument_tys.is_empty()) {
        // Simple case, no args expected, none provided - OK
        (None, true) => {
            ctx.values_mut()
                .insert(expr_id, ValueResolution::simple(case.metadata().id()));
            Ok(SolveResult::Solved)
        },

        // Simple case but args provided - error
        (None, false) => Err(InferenceError::member_not_found(
            resolved_ty.clone(),
            format!("{}(...)", member_name),
            span.clone(),
        )),

        // Case with params, no args provided - error (unless params are empty)
        (Some(cb), true) if !cb.parameters().is_empty() => Err(InferenceError::member_not_found(
            resolved_ty.clone(),
            member_name.to_string(),
            span.clone(),
        )),

        // Case with empty params, no args - OK
        (Some(_cb), true) => {
            ctx.values_mut()
                .insert(expr_id, ValueResolution::simple(case.metadata().id()));
            Ok(SolveResult::Solved)
        },

        // Case with params, args provided - validate
        (Some(cb), false) => {
            let params = cb.parameters();

            // Check arity
            if params.len() != argument_tys.len() {
                return Err(InferenceError::closure_arity_mismatch(
                    argument_tys.len(),
                    params.len(),
                    span.clone(),
                ));
            }

            // Check labels and equate types
            let mut labels_match = true;
            let provided_labels: Vec<Option<String>> = argument_tys
                .iter()
                .map(|(label, _)| label.clone())
                .collect();
            let expected_labels: Vec<Option<String>> = params
                .iter()
                .map(|p| p.label.as_ref().map(|l| l.value.clone()))
                .collect();

            for ((label, _), param) in argument_tys.iter().zip(params.iter()) {
                let expected_label = param.label.as_ref().map(|l| l.value.as_str());
                let actual_label = label.as_deref();

                // Labels must match exactly: if param has label, arg must provide it
                if actual_label != expected_label {
                    labels_match = false;
                    break;
                }
            }

            if !labels_match {
                return Err(InferenceError::no_matching_overload(
                    member_name.to_string(),
                    resolved_ty.clone(),
                    provided_labels,
                    expected_labels,
                    span.clone(),
                ));
            }

            // All labels match - equate types
            for ((_, arg_ty_id), param) in argument_tys.iter().zip(params.iter()) {
                // Apply substitutions to parameter type and equate
                let param_ty = param.ty.apply_substitutions(substitutions);
                ctx.register_type(&param_ty);
                ctx.equate(*arg_ty_id, param_ty.id(), span.clone());
            }

            ctx.values_mut()
                .insert(expr_id, ValueResolution::simple(case.metadata().id()));
            Ok(SolveResult::Solved)
        },
    }
}

/// Resolve an enum pattern binding constraint.
///
/// This connects the types of bindings in an enum pattern (like `.Some(value)`)
/// to the corresponding parameter types of the enum case.
fn resolve_enum_pattern_binding(
    ctx: &mut InferenceContext<'_>,
    enum_ty: TyId,
    case_name: &str,
    binding_tys: &[(Option<String>, TyId)],
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    #[allow(unused_imports)]
    use kestrel_semantic_tree::behavior::callable::CallableBehavior;
    use semantic_tree::symbol::Symbol;

    let resolved_ty = resolve_type(ctx, enum_ty);

    // Expand type aliases to get underlying type (e.g., OptionalTypeOperator -> Optional)
    let resolved_ty = resolved_ty.expand_aliases();

    // If still Infer, defer until the enum type is known
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Must be an enum type
    let TyKind::Enum {
        symbol: enum_symbol,
        substitutions,
    } = resolved_ty.kind()
    else {
        // Not an enum type - this shouldn't happen in well-formed code,
        // but we can just skip the constraint (type mismatch will be caught elsewhere)
        return Ok(SolveResult::Solved);
    };

    // Find the case by name
    let cases = enum_symbol.cases();
    let case = cases
        .iter()
        .find(|c| c.metadata().name().value == case_name);

    let Some(case) = case else {
        // Case not found - emit an error
        let enum_name = enum_symbol.metadata().name().value.clone();
        return Err(InferenceError::unknown_enum_case(
            enum_name,
            case_name.to_string(),
            span.clone(),
        ));
    };

    // Get the callable behavior (parameter types) if the case has associated values
    let Some(callable) = case.callable_behavior() else {
        // Case has no parameters, but we have bindings - this shouldn't happen
        // in well-formed patterns, but we can skip
        return Ok(SolveResult::Solved);
    };

    let params = callable.parameters();

    // Match bindings to parameters
    // For positional bindings (no label), match by position
    // For labeled bindings, match by label
    for (idx, (label, binding_ty_id)) in binding_tys.iter().enumerate() {
        let param = if let Some(label_name) = label {
            // Labeled binding - find parameter by label
            params.iter().find(|p| {
                p.external_label() == Some(label_name.as_str())
                    || p.internal_name() == label_name.as_str()
            })
        } else {
            // Positional binding - use index
            params.get(idx)
        };

        if let Some(param) = param {
            // Apply substitutions to the parameter type and equate
            let param_ty = param.ty.apply_substitutions(substitutions);
            ctx.register_type(&param_ty);
            ctx.equate(*binding_ty_id, param_ty.id(), span.clone());
        }
    }

    Ok(SolveResult::Solved)
}

/// Resolve a struct pattern binding constraint.
///
/// This connects the types of bindings in a struct pattern (like `Point { x, y }`)
/// to the corresponding field types of the struct.
fn resolve_struct_pattern_binding(
    ctx: &mut InferenceContext<'_>,
    struct_ty: TyId,
    struct_name: &str,
    field_bindings: &[(String, TyId)],
    has_rest: bool,
    span: &Span,
) -> Result<SolveResult, InferenceError> {
    use kestrel_semantic_tree::behavior::typed::TypedBehavior;
    use kestrel_semantic_tree::symbol::field::FieldSymbol;
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
    use semantic_tree::symbol::Symbol;

    let resolved_ty = resolve_type(ctx, struct_ty);

    // If still Infer, defer until the struct type is known
    if matches!(resolved_ty.kind(), TyKind::Infer) {
        return Ok(SolveResult::Deferred);
    }

    // Must be a struct type
    let TyKind::Struct {
        symbol: struct_symbol,
        substitutions,
    } = resolved_ty.kind()
    else {
        // Not a struct type - this shouldn't happen in well-formed code,
        // but we can just skip the constraint (type mismatch will be caught elsewhere)
        return Ok(SolveResult::Solved);
    };

    // Get field symbols from struct
    let fields: Vec<_> = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
        .filter_map(|c| c.downcast_arc::<FieldSymbol>().ok())
        .collect();

    // Collect field names from the struct
    let struct_field_names: std::collections::HashSet<_> = fields
        .iter()
        .map(|f| f.metadata().name().value.clone())
        .collect();

    // Match bindings to fields by name
    for (field_name, binding_ty_id) in field_bindings {
        let field = fields
            .iter()
            .find(|f| &f.metadata().name().value == field_name);

        if let Some(field) = field {
            // Get field type from TypedBehavior (resolved type) or fallback to field_type
            let raw_field_ty = field
                .metadata()
                .get_behavior::<TypedBehavior>()
                .map(|typed| typed.ty().clone())
                .unwrap_or_else(|| field.field_type().clone());

            // Apply substitutions to handle generic structs
            let field_ty = raw_field_ty.apply_substitutions(substitutions);
            ctx.register_type(&field_ty);
            ctx.equate(*binding_ty_id, field_ty.id(), span.clone());
        } else {
            // Unknown field error
            return Err(InferenceError::unknown_struct_field(
                struct_name.to_string(),
                field_name.clone(),
                span.clone(),
            ));
        }
    }

    // Check for missing fields if no rest pattern
    if !has_rest {
        let matched_field_names: std::collections::HashSet<_> = field_bindings
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        let missing: Vec<_> = struct_field_names
            .difference(&matched_field_names)
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(InferenceError::missing_struct_fields(
                struct_name.to_string(),
                missing,
                span.clone(),
            ));
        }
    }

    Ok(SolveResult::Solved)
}

/// Follow the substitution chain to get the current resolved type for an ID.
fn resolve_type(ctx: &InferenceContext<'_>, id: TyId) -> Ty {
    // Follow substitution chain, tracking the last concrete type found
    let mut current_id = id;
    let mut last_subst: Option<Ty> = None;
    let mut visited = HashSet::new();

    loop {
        if !visited.insert(current_id) {
            // Cycle detected - return what we have
            break;
        }

        if let Some(subst) = ctx.substitutions().get(&current_id) {
            last_subst = Some(subst.clone());
            current_id = subst.id();
        } else {
            break;
        }
    }

    // Return the last substitution found (which is the resolved type),
    // or look in registry, or create inference placeholder
    last_subst
        .or_else(|| ctx.type_registry().get(&current_id).cloned())
        .unwrap_or_else(|| {
            // If not found anywhere, create an inference placeholder
            Ty::infer(Span::new(0, 0..0))
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
    if let Some(subst) = ctx.substitutions().get(&ty.id())
        && occurs_check_inner(var, subst, ctx, visited)
    {
        return true;
    }

    // Recursively check compound types
    match ty.kind() {
        TyKind::Tuple(elements) => elements
            .iter()
            .any(|e| occurs_check_inner(var, e, ctx, visited)),
        // Note: Array[T] struct types have their substitutions checked via the Struct case
        TyKind::Pointer(elem) => occurs_check_inner(var, elem, ctx, visited),
        TyKind::Function {
            params,
            return_type,
        } => {
            params
                .iter()
                .any(|p| occurs_check_inner(var, p, ctx, visited))
                || occurs_check_inner(var, return_type, ctx, visited)
        },
        TyKind::Struct { substitutions, .. }
        | TyKind::Enum { substitutions, .. }
        | TyKind::Protocol { substitutions, .. }
        | TyKind::TypeAlias { substitutions, .. } => substitutions
            .iter()
            .any(|(_, t)| occurs_check_inner(var, t, ctx, visited)),
        TyKind::AssociatedType { container, .. } => container
            .as_ref()
            .map(|c| occurs_check_inner(var, c, ctx, visited))
            .unwrap_or(false),
        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            if occurs_check_inner(var, return_type, ctx, visited) {
                return true;
            }
            match param_info {
                ParamInfo::ImplicitIt { it_type } => occurs_check_inner(var, it_type, ctx, visited),
                ParamInfo::Explicit { param_types } => param_types
                    .iter()
                    .any(|p| occurs_check_inner(var, p, ctx, visited)),
                ParamInfo::Unconstrained => false,
            }
        },
        // Leaf types
        _ => false,
    }
}

/// Check that all inference placeholders have been resolved.
/// If any remain unresolved, adds an Ambiguous error to the context.
///
/// Also processes any remaining constraints that may now be solvable
/// (e.g., ImplicitMember constraints that were deferred waiting for type info).
fn check_fully_resolved(ctx: &mut InferenceContext<'_>) {
    // First, try to solve any remaining constraints one more time.
    // This handles cases where constraints were deferred but the types
    // have since been resolved through other unification.
    let remaining_constraints = ctx.take_constraints();
    for constraint in remaining_constraints {
        match try_solve(ctx, &constraint) {
            Ok(SolveResult::Solved) => {
                // Great, constraint is now solved
            },
            Ok(SolveResult::Deferred) => {
                // Still can't solve - check if it's an ImplicitMember that we can
                // report a better error for
                if let Constraint::ImplicitMember {
                    member_name, span, ..
                } = &constraint
                {
                    // Report specific error for unresolved enum shorthand
                    ctx.add_error(InferenceError::cannot_infer_enum_type(
                        member_name.clone(),
                        span.clone(),
                    ));
                } else {
                    // Put it back for generic error checking below
                    ctx.push_constraint(constraint);
                }
            },
            Err(error) => {
                // Constraint failed - record the error
                ctx.add_error(error);
            },
        }
    }

    let mut unresolved = Vec::new();

    // Check all registered types
    for (id, ty) in ctx.type_registry() {
        if matches!(ty.kind(), TyKind::Infer) && !ctx.substitutions().contains_key(id) {
            unresolved.push((*id, ty.span().clone()));
        }
    }

    // Check for any inference placeholders in remaining constraints
    for constraint in ctx.constraints() {
        match constraint {
            Constraint::Equals { a, b, .. } => {
                check_resolved_id(*a, ctx, &mut unresolved);
                check_resolved_id(*b, ctx, &mut unresolved);
            },
            Constraint::Conforms { ty, .. } => {
                check_resolved_id(*ty, ctx, &mut unresolved);
            },
            Constraint::Normalizes { base, result, .. } => {
                check_resolved_id(*base, ctx, &mut unresolved);
                check_resolved_id(*result, ctx, &mut unresolved);
            },
            Constraint::MemberAccess {
                receiver, result, ..
            } => {
                check_resolved_id(*receiver, ctx, &mut unresolved);
                check_resolved_id(*result, ctx, &mut unresolved);
            },
            Constraint::ImplicitMember {
                expr_ty,
                argument_tys,
                ..
            } => {
                check_resolved_id(*expr_ty, ctx, &mut unresolved);
                for (_, arg_ty) in argument_tys {
                    check_resolved_id(*arg_ty, ctx, &mut unresolved);
                }
            },
            Constraint::EnumPatternBinding {
                enum_ty,
                binding_tys,
                ..
            } => {
                check_resolved_id(*enum_ty, ctx, &mut unresolved);
                for (_, binding_ty) in binding_tys {
                    check_resolved_id(*binding_ty, ctx, &mut unresolved);
                }
            },
            Constraint::StructPatternBinding {
                struct_ty,
                field_bindings,
                ..
            } => {
                check_resolved_id(*struct_ty, ctx, &mut unresolved);
                for (_, binding_ty) in field_bindings {
                    check_resolved_id(*binding_ty, ctx, &mut unresolved);
                }
            },
            Constraint::Promotable { from_ty, to_ty, .. } => {
                check_resolved_id(*from_ty, ctx, &mut unresolved);
                check_resolved_id(*to_ty, ctx, &mut unresolved);
            },
            Constraint::TupleIndexAccess { tuple, result, .. } => {
                check_resolved_id(*tuple, ctx, &mut unresolved);
                check_resolved_id(*result, ctx, &mut unresolved);
            },
        }
    }

    if !unresolved.is_empty() {
        // Deduplicate by TyId
        unresolved.sort_by_key(|(id, _)| id.raw());
        unresolved.dedup_by_key(|(id, _)| id.raw());
        ctx.add_error(InferenceError::ambiguous(unresolved));
    }
}

fn check_resolved_id(id: TyId, ctx: &InferenceContext<'_>, unresolved: &mut Vec<(TyId, Span)>) {
    let ty = resolve_type(ctx, id);
    if matches!(ty.kind(), TyKind::Infer | TyKind::UnresolvedFunction { .. }) {
        unresolved.push((id, ty.span().clone()));
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
                receiver_ty: Ty::unit(Span::new(0, 0..0)),
                member: String::new(),
            })
        }

        fn conforms_to(&self, _ty: &Ty, _protocol_id: SymbolId) -> bool {
            false
        }

        fn resolve_associated_type(&self, _container: &Ty, _assoc_name: &str) -> Option<Ty> {
            None
        }

        fn symbol_name(&self, _symbol_id: SymbolId) -> Option<String> {
            None
        }

        fn builtin_protocol(&self, _feature: LanguageFeature) -> Option<SymbolId> {
            None
        }

        fn default_array_type(&self, _element_ty: Ty, _span: Span) -> Option<Ty> {
            None
        }
    }

    #[test]
    fn test_unify_same_primitive() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let ty1 = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::new(0, 0..3));
        let ty2 = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::new(0, 4..7));

        ctx.register_type(&ty1);
        ctx.register_type(&ty2);
        ctx.equate(ty1.id(), ty2.id(), Span::new(0, 0..7));

        let solution = ctx.solve();
        assert!(!solution.has_errors());
    }

    #[test]
    fn test_unify_infer_with_concrete() {
        let oracle = TestOracle;
        let mut ctx = InferenceContext::new(&oracle);

        let infer_ty = Ty::infer(Span::new(0, 0..1));
        let concrete_ty = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::new(0, 2..5));

        ctx.register_type(&infer_ty);
        ctx.register_type(&concrete_ty);
        ctx.equate(infer_ty.id(), concrete_ty.id(), Span::new(0, 0..5));

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

        let int_ty = Ty::int(kestrel_semantic_tree::ty::IntBits::I64, Span::new(0, 0..3));
        let string_ty = Ty::string(Span::new(0, 4..10));

        ctx.register_type(&int_ty);
        ctx.register_type(&string_ty);
        ctx.equate(int_ty.id(), string_ty.id(), Span::new(0, 0..10));

        let solution = ctx.solve();
        assert!(solution.has_errors());
        assert!(matches!(
            &solution.errors()[0],
            InferenceError::TypeMismatch { .. }
        ));
    }
}
