//! Match expression lowering.
//!
//! This module handles lowering of match expressions to MIR using decision trees
//! compiled from the pattern matching analysis crate.
//!
//! # Algorithm Overview
//!
//! 1. Extract patterns and guards from match arms
//! 2. Compile patterns into a decision tree using Maranget's algorithm
//! 3. Lower the scrutinee to a MIR place
//! 4. Emit the decision tree as MIR control flow:
//!    - Switch nodes become MIR switch terminators (for enums) or branches (for bools)
//!    - Success nodes emit bindings and arm bodies
//!    - Guard nodes emit condition checks with fallback
//!
//! # MIR Structure
//!
//! A match expression produces the following MIR structure:
//!
//! ```text
//! entry_block:
//!     %scrutinee = <lower scrutinee expression>
//!     <emit decision tree>
//!
//! arm_0_block:
//!     <emit bindings for arm 0>
//!     %arm_0_result = <lower arm 0 body>
//!     %result = copy %arm_0_result
//!     jump join_block
//!
//! arm_1_block:
//!     ...
//!
//! join_block:
//!     // result is in %result
//! ```

use kestrel_execution_graph::{
    BinOp, CallArg, Callee, Immediate, Place, QualifiedNameData, Rvalue, Value,
};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_pattern_matching::{
    AccessPath, Binding, Constructor, DecisionTree, PathElement, compile,
};
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::expr::{Expression, MatchArm};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::name::qualified_name_for_symbol;
use crate::ty::{lower_type, make_int_immediate};

/// Lower a match expression to MIR.
///
/// # Arguments
///
/// * `ctx` - The lowering context
/// * `scrutinee` - The expression being matched
/// * `arms` - The match arms
/// * `expr` - The full match expression (for type and span info)
///
/// # Returns
///
/// A MIR Value representing the result of the match expression.
pub fn lower_match_expr(
    ctx: &mut LoweringContext,
    scrutinee: &Expression,
    arms: &[MatchArm],
    expr: &Expression,
) -> Value {
    // Get result type and create result place
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("match_result", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if the result type needs deinit
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Lower the scrutinee
    let scrutinee_value = lower_expression(ctx, scrutinee);

    // We need the scrutinee in a place for pattern matching
    let scrutinee_place = match scrutinee_value {
        Value::Place(p) => p,
        Value::Immediate(imm) => {
            // Store the immediate in a temporary
            let scrutinee_ty = lower_type(ctx, &scrutinee.ty);
            let scrutinee_local = ctx.create_temp("scrutinee", scrutinee_ty);
            let place = Place::local(scrutinee_local);
            ctx.emit_assign(place.clone(), Rvalue::Use(imm));
            place
        },
        Value::Unreachable => {
            // Scrutinee diverged (e.g., was a return), so match is unreachable
            return Value::Unreachable;
        },
    };

    // Extract patterns and guards from arms
    let patterns: Vec<_> = arms.iter().map(|arm| arm.pattern.clone()).collect();
    let has_guards: Vec<_> = arms.iter().map(|arm| arm.guard.is_some()).collect();

    // Compile the decision tree
    let decision_tree = compile(&patterns, &scrutinee.ty, &has_guards);

    // Create the join block where all arms converge
    let join_block = ctx.create_block();

    // Emit the decision tree
    emit_decision_tree(
        ctx,
        &decision_tree,
        &scrutinee_place,
        arms,
        &result_place,
        join_block,
    );

    // Continue with the join block
    ctx.set_current_block(join_block);

    Value::Place(result_place)
}

/// Emit MIR for a decision tree node.
fn emit_decision_tree(
    ctx: &mut LoweringContext,
    tree: &DecisionTree,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    match tree {
        DecisionTree::Success {
            arm_index,
            bindings,
        } => {
            emit_success(
                ctx,
                *arm_index,
                bindings,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_switch(
                ctx,
                path,
                ty,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        DecisionTree::Guard {
            arm_index,
            bindings,
            success,
            failure,
        } => {
            emit_guard(
                ctx,
                *arm_index,
                bindings,
                success,
                failure,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        DecisionTree::Failure => {
            // Unreachable - exhaustiveness checking should prevent this
            ctx.emit_unreachable();
        },
    }
}

/// Emit MIR for a successful match (pattern matched, no more tests needed).
fn emit_success(
    ctx: &mut LoweringContext,
    arm_index: usize,
    bindings: &[Binding],
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Emit bindings
    emit_bindings(ctx, bindings, scrutinee);

    // Get the arm body
    let arm = &arms[arm_index];

    // Lower the arm body
    let body_value = lower_expression(ctx, &arm.body);

    // If the block is already terminated (e.g., by return/break), don't emit more
    if ctx.is_block_terminated() {
        return;
    }

    // Assign result - use emit_move_value to mark the temp as moved, preventing double-free
    ctx.emit_move_value(result_place.clone(), body_value);

    // Jump to join block
    ctx.emit_jump(join_block);
}

/// Emit MIR for pattern bindings.
///
/// This is public so it can be reused by if-let lowering.
pub fn emit_bindings(ctx: &mut LoweringContext, bindings: &[Binding], scrutinee: &Place) {
    for binding in bindings {
        // Compute the place for this binding
        let source_place = apply_path(scrutinee, &binding.path);

        // Create the local and map it
        let binding_ty = lower_type(ctx, &binding.ty);
        let local = ctx.create_local(&binding.name, binding_ty);
        ctx.map_local(binding.local_id, local);

        // Copy the value to the local
        let local_place = Place::local(local);
        ctx.emit_copy(local_place, source_place);
    }
}

/// Apply a path to a place to get a sub-place.
///
/// This is public so it can be reused by if-let lowering.
pub fn apply_path(base: &Place, path: &AccessPath) -> Place {
    let mut result = base.clone();
    for elem in path {
        result = match elem {
            PathElement::Field(name) => result.field(name),
            PathElement::Index(idx) => result.index(*idx),
            PathElement::Downcast(variant) => result.downcast(variant),
        };
    }
    result
}

/// Emit MIR for a switch node in the decision tree.
fn emit_switch(
    ctx: &mut LoweringContext,
    path: &AccessPath,
    ty: &Ty,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Get the place to switch on
    let switch_place = apply_path(scrutinee, path);

    match ty.kind() {
        TyKind::Bool => {
            emit_bool_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        TyKind::Enum { .. } => {
            emit_enum_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        TyKind::Int(int_bits) => {
            emit_int_switch(
                ctx,
                &switch_place,
                *int_bits,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        TyKind::String => {
            emit_string_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },

        TyKind::Tuple(_) => {
            // Tuples have a single constructor - just recurse into the single case
            if let Some((_, subtree)) = cases.first() {
                emit_decision_tree(ctx, subtree, scrutinee, arms, result_place, join_block);
            } else if let Some(default_tree) = default {
                emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
            } else {
                ctx.emit_unreachable();
            }
        },

        TyKind::Struct { .. } => {
            // For structs with literal constructors, check if they conform to Matchable
            if has_literal_constructors(cases) {
                if type_conforms_to_matchable(ctx, ty) {
                    // Use the Matchable protocol for comparison
                    emit_matchable_switch(
                        ctx,
                        &switch_place,
                        ty,
                        cases,
                        default,
                        scrutinee,
                        arms,
                        result_place,
                        join_block,
                    );
                } else {
                    // Fall back to comparison chain
                    emit_comparison_chain(
                        ctx,
                        &switch_place,
                        ty,
                        cases,
                        default,
                        scrutinee,
                        arms,
                        result_place,
                        join_block,
                    );
                }
            } else {
                // Structs have a single constructor - just recurse
                if let Some((_, subtree)) = cases.first() {
                    emit_decision_tree(ctx, subtree, scrutinee, arms, result_place, join_block);
                } else if let Some(default_tree) = default {
                    emit_decision_tree(
                        ctx,
                        default_tree,
                        scrutinee,
                        arms,
                        result_place,
                        join_block,
                    );
                } else {
                    ctx.emit_unreachable();
                }
            }
        },

        _ => {
            // For other types, emit a comparison chain
            emit_comparison_chain(
                ctx,
                &switch_place,
                ty,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },
    }
}

/// Emit MIR for a boolean switch.
fn emit_bool_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Find true and false cases
    let true_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::True))
        .map(|(_, t)| t);
    let false_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::False))
        .map(|(_, t)| t);

    // Create blocks for each case
    let true_block = ctx.create_block();
    let false_block = ctx.create_block();

    // Emit branch
    ctx.emit_branch(Value::Place(switch_place.clone()), true_block, false_block);

    // Emit true case
    ctx.set_current_block(true_block);
    if let Some(tree) = true_tree {
        emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
    } else if let Some(default_tree) = default {
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    } else {
        ctx.emit_unreachable();
    }

    // Emit false case
    ctx.set_current_block(false_block);
    if let Some(tree) = false_tree {
        emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
    } else if let Some(default_tree) = default {
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    } else {
        ctx.emit_unreachable();
    }
}

/// Emit MIR for an enum switch.
fn emit_enum_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Build switch cases: (variant_name, block)
    let mut switch_cases = Vec::with_capacity(cases.len());
    let mut case_trees = Vec::with_capacity(cases.len());

    for (ctor, tree) in cases {
        if let Constructor::Variant { name, .. } = ctor {
            let case_block = ctx.create_block();
            switch_cases.push((name.clone(), case_block));
            case_trees.push((case_block, tree));
        }
    }

    // Add default case if present
    let default_block = if default.is_some() {
        let block = ctx.create_block();
        // Add a wildcard case that catches everything else
        // For now we use "_" as the default case name
        switch_cases.push(("_".to_string(), block));
        Some(block)
    } else {
        None
    };

    // Emit the switch terminator
    ctx.emit_switch(switch_place.clone(), switch_cases);

    // Emit each case's body
    for (block, tree) in case_trees {
        ctx.set_current_block(block);
        emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
    }

    // Emit default case body if present
    if let (Some(block), Some(default_tree)) = (default_block, default) {
        ctx.set_current_block(block);
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    }
}

/// Emit MIR for an integer switch (comparison chain).
fn emit_int_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    int_bits: kestrel_semantic_tree::ty::IntBits,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    emit_comparison_chain_int(
        ctx,
        switch_place,
        int_bits,
        cases,
        default,
        scrutinee,
        arms,
        result_place,
        join_block,
    );
}

/// Emit MIR for a string switch (comparison chain).
fn emit_string_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    emit_comparison_chain_string(
        ctx,
        switch_place,
        cases,
        default,
        scrutinee,
        arms,
        result_place,
        join_block,
    );
}

/// Emit a comparison chain for integer literals.
fn emit_comparison_chain_int(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    int_bits: kestrel_semantic_tree::ty::IntBits,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // If no cases, go to default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
        } else {
            ctx.emit_unreachable();
        }
        return;
    }

    // Build a chain of comparisons
    for (i, (ctor, tree)) in cases.iter().enumerate() {
        let is_last = i == cases.len() - 1;

        match ctor {
            Constructor::IntLiteral(value) => {
                // Create blocks
                let match_block = ctx.create_block();
                let next_block = if is_last && default.is_none() {
                    // Last case with no default - unreachable if not matched
                    ctx.create_block()
                } else {
                    ctx.create_block()
                };

                // Compare: switch_place == value
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::Eq,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(make_int_immediate(int_bits, *value)),
                    },
                );

                // Branch
                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::IntRange { start, end } => {
                // Range check: start <= switch_place && switch_place <= end
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // start <= value
                let cmp1_ty = ctx.mir.ty_bool();
                let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                let cmp1_place = Place::local(cmp1_local);
                ctx.emit_assign(
                    cmp1_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Immediate(make_int_immediate(int_bits, *start)),
                        rhs: Value::Place(switch_place.clone()),
                    },
                );

                // value <= end
                let cmp2_ty = ctx.mir.ty_bool();
                let cmp2_local = ctx.create_temp("cmp_hi", cmp2_ty);
                let cmp2_place = Place::local(cmp2_local);
                ctx.emit_assign(
                    cmp2_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(make_int_immediate(int_bits, *end)),
                    },
                );

                // cmp1 && cmp2
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp_range", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::BoolAnd,
                        lhs: Value::Place(cmp1_place),
                        rhs: Value::Place(cmp2_place),
                    },
                );

                // Branch
                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::CharLiteral(c) => {
                // Char literals are just integers - treat as i32 comparison
                let match_block = ctx.create_block();
                let next_block = if is_last && default.is_none() {
                    ctx.create_block()
                } else {
                    ctx.create_block()
                };

                // Compare: switch_place == char value
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::Eq,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(make_int_immediate(int_bits, *c as i64)),
                    },
                );

                // Branch
                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::CharRange { start, end } => {
                // Char range check: start <= switch_place && switch_place <= end
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // start <= value
                let cmp1_ty = ctx.mir.ty_bool();
                let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                let cmp1_place = Place::local(cmp1_local);
                ctx.emit_assign(
                    cmp1_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Immediate(make_int_immediate(int_bits, *start as i64)),
                        rhs: Value::Place(switch_place.clone()),
                    },
                );

                // value <= end
                let cmp2_ty = ctx.mir.ty_bool();
                let cmp2_local = ctx.create_temp("cmp_hi", cmp2_ty);
                let cmp2_place = Place::local(cmp2_local);
                ctx.emit_assign(
                    cmp2_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(make_int_immediate(int_bits, *end as i64)),
                    },
                );

                // cmp1 && cmp2
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp_range", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::BoolAnd,
                        lhs: Value::Place(cmp1_place),
                        rhs: Value::Place(cmp2_place),
                    },
                );

                // Branch
                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            _ => {
                // Unsupported constructor for int switch
                ctx.emit_error(LoweringError::internal(
                    format!("unsupported constructor in int switch: {:?}", ctor),
                    None,
                ));
                ctx.emit_unreachable();
                return;
            },
        }
    }

    // After all cases, emit default
    if let Some(default_tree) = default {
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    } else {
        ctx.emit_unreachable();
    }
}

/// Emit a comparison chain for string literals.
fn emit_comparison_chain_string(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // If no cases, go to default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
        } else {
            ctx.emit_unreachable();
        }
        return;
    }

    // Build a chain of string comparisons
    for (ctor, tree) in cases.iter() {
        if let Constructor::StringLiteral(value) = ctor {
            let match_block = ctx.create_block();
            let next_block = ctx.create_block();

            // Compare: switch_place == value (using string comparison)
            let cmp_ty = ctx.mir.ty_bool();
            let cmp_local = ctx.create_temp("cmp", cmp_ty);
            let cmp_place = Place::local(cmp_local);
            ctx.emit_assign(
                cmp_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::StrEq,
                    lhs: Value::Place(switch_place.clone()),
                    rhs: Value::Immediate(Immediate::string(value.clone())),
                },
            );

            // Branch
            ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

            // Emit match body
            ctx.set_current_block(match_block);
            emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

            // Continue with next comparison
            ctx.set_current_block(next_block);
        }
    }

    // After all cases, emit default
    if let Some(default_tree) = default {
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    } else {
        ctx.emit_unreachable();
    }
}

/// Generic comparison chain for other types.
fn emit_comparison_chain(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    ty: &Ty,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // For unsupported types, just try int comparison chain as fallback
    match ty.kind() {
        TyKind::Int(int_bits) => {
            emit_comparison_chain_int(
                ctx,
                switch_place,
                *int_bits,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },
        TyKind::String => {
            emit_comparison_chain_string(
                ctx,
                switch_place,
                cases,
                default,
                scrutinee,
                arms,
                result_place,
                join_block,
            );
        },
        _ => {
            // Fallback: just emit default or first case
            if let Some(default_tree) = default {
                emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
            } else if let Some((_, tree)) = cases.first() {
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
            } else {
                ctx.emit_unreachable();
            }
        },
    }
}

/// Emit MIR for a guard node.
fn emit_guard(
    ctx: &mut LoweringContext,
    arm_index: usize,
    bindings: &[Binding],
    success: &DecisionTree,
    failure: &DecisionTree,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    let arm = &arms[arm_index];

    // First, emit the bindings so the guard can reference them
    emit_bindings(ctx, bindings, scrutinee);

    // Lower the guard expression
    let guard_expr = arm
        .guard
        .as_ref()
        .expect("Guard arm should have guard expression");
    let guard_value = lower_expression(ctx, guard_expr);

    // Create blocks for success and failure
    let success_block = ctx.create_block();
    let failure_block = ctx.create_block();

    // Branch on the guard
    ctx.emit_branch(guard_value, success_block, failure_block);

    // Emit success path
    ctx.set_current_block(success_block);
    emit_decision_tree(ctx, success, scrutinee, arms, result_place, join_block);

    // Emit failure path
    ctx.set_current_block(failure_block);
    emit_decision_tree(ctx, failure, scrutinee, arms, result_place, join_block);
}

/// Check if a type conforms to the Matchable protocol.
fn type_conforms_to_matchable(ctx: &LoweringContext, ty: &Ty) -> bool {
    use semantic_tree::symbol::Symbol;

    // Get the Matchable protocol ID from the builtin registry
    let Some(matchable_id) = ctx.model.builtin_registry().matchable_protocol() else {
        return false;
    };

    // Helper to check conformances on a symbol
    fn check_conformances(
        symbol: &dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
        matchable_id: semantic_tree::symbol::SymbolId,
    ) -> bool {
        if let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() {
            for conf in conformances.conformances() {
                if let TyKind::Protocol {
                    symbol: conf_proto, ..
                } = conf.kind()
                    && conf_proto.metadata().id() == matchable_id
                {
                    return true;
                }
            }
        }
        false
    }

    // Check if the type is a struct or enum that conforms to Matchable
    match ty.kind() {
        TyKind::Struct { symbol, .. } => check_conformances(symbol.as_ref(), matchable_id),
        TyKind::Enum { symbol, .. } => check_conformances(symbol.as_ref(), matchable_id),
        _ => false,
    }
}

/// Check if any of the constructors in the cases are literals that would
/// require comparison (IntLiteral, StringLiteral, etc.)
fn has_literal_constructors(cases: &[(Constructor, DecisionTree)]) -> bool {
    cases.iter().any(|(ctor, _)| {
        matches!(
            ctor,
            Constructor::IntLiteral(_)
                | Constructor::StringLiteral(_)
                | Constructor::CharLiteral(_)
                | Constructor::IntRange { .. }
                | Constructor::CharRange { .. }
        )
    })
}

/// Collect the qualified name parts for a symbol (namespace hierarchy).
fn collect_symbol_name_parts(
    symbol: &std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
    parts: &mut Vec<String>,
) {
    // First, collect parent segments
    if let Some(parent) = symbol.metadata().parent() {
        collect_symbol_name_parts(&parent, parts);
    }

    // Then add this symbol's name
    let kind = symbol.metadata().kind();
    let name_value = &symbol.metadata().name().value;

    // Skip root
    if name_value == "<root>" {
        return;
    }

    match kind {
        KestrelSymbolKind::SourceFile => {},
        KestrelSymbolKind::Module
        | KestrelSymbolKind::Struct
        | KestrelSymbolKind::Enum
        | KestrelSymbolKind::Protocol
        | KestrelSymbolKind::TypeAlias
        | KestrelSymbolKind::Extension => {
            parts.push(name_value.clone());
        },
        _ => {},
    }
}

/// Emit MIR for a matchable switch using the Matchable protocol.
fn emit_matchable_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    ty: &Ty,
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // If no cases, go to default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
        } else {
            ctx.emit_unreachable();
        }
        return;
    }

    // Get the Matchable protocol for witness calls
    let Some(matchable_id) = ctx.model.builtin_registry().matchable_protocol() else {
        ctx.emit_error(LoweringError::internal(
            "Matchable protocol not found in registry",
            None,
        ));
        ctx.emit_unreachable();
        return;
    };

    let Some(matchable_symbol) = ctx.model.query(SymbolFor { id: matchable_id }) else {
        ctx.emit_error(LoweringError::internal(
            "Matchable protocol symbol not found",
            None,
        ));
        ctx.emit_unreachable();
        return;
    };

    // Find the `matches` method in the Matchable protocol to get its return type
    let matches_method = matchable_symbol
        .metadata()
        .children()
        .into_iter()
        .find(|child| {
            child.metadata().kind() == KestrelSymbolKind::Function
                && child.metadata().name().value == "matches"
        });

    let bool_mir_ty = if let Some(method) = matches_method {
        if let Some(callable) = method.metadata().get_behavior::<CallableBehavior>() {
            // Lower the return type to MIR
            lower_type(ctx, callable.return_type())
        } else {
            // Fallback - shouldn't happen
            ctx.mir.ty_bool()
        }
    } else {
        // Fallback - shouldn't happen
        ctx.mir.ty_bool()
    };

    let protocol_name = qualified_name_for_symbol(ctx, &matchable_symbol);
    let for_type = lower_type(ctx, ty);

    // Build a chain of Matchable.matches() calls
    for (i, (ctor, tree)) in cases.iter().enumerate() {
        let is_last = i == cases.len() - 1;

        // Handle each constructor type
        match ctor {
            Constructor::IntLiteral(value) => {
                // Create blocks for this case
                let match_block = ctx.create_block();
                let next_block = if is_last && default.is_none() {
                    // Last case with no default - unreachable if not matched
                    ctx.create_block()
                } else {
                    ctx.create_block()
                };

                // Create a temporary to hold the literal value
                // The type must conform to ExpressibleByIntLiteral
                let literal_local = ctx.create_temp("literal", for_type);
                let literal_place = Place::local(literal_local);

                // Initialize the struct from the literal using the ExpressibleByIntLiteral init
                // This matches the pattern from lower_literal_init_call in expr.rs
                if let TyKind::Struct {
                    symbol: struct_symbol,
                    ..
                } = ty.kind()
                {
                    // Find the init with the intLiteral label
                    let init_symbol =
                        struct_symbol
                            .metadata()
                            .children()
                            .into_iter()
                            .find(|child| {
                                if child.metadata().kind() != KestrelSymbolKind::Initializer {
                                    return false;
                                }
                                // Check if this init has a parameter with the intLiteral label
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    callable.parameters().first().is_some_and(|p| {
                                        p.label.as_ref().is_some_and(|l| l.value == "intLiteral")
                                    })
                                } else {
                                    false
                                }
                            });

                    if let Some(init_sym) = init_symbol {
                        // Build the qualified name for the init function
                        let mut name_parts = Vec::new();
                        collect_symbol_name_parts(
                            &(struct_symbol.clone()
                                as std::sync::Arc<
                                    dyn semantic_tree::symbol::Symbol<
                                            kestrel_semantic_tree::language::KestrelLanguage,
                                        >,
                                >),
                            &mut name_parts,
                        );

                        // Get all labels from the found init symbol
                        let init_name_suffix = if let Some(callable) =
                            init_sym.metadata().get_behavior::<CallableBehavior>()
                        {
                            let labels: Vec<&str> = callable
                                .parameters()
                                .iter()
                                .filter_map(|p| p.external_label())
                                .collect();
                            if labels.is_empty() {
                                "init".to_string()
                            } else {
                                format!("init${}", labels.join("$"))
                            }
                        } else {
                            "init$intLiteral".to_string()
                        };
                        name_parts.push(init_name_suffix);

                        let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

                        // Create a mutable reference to the result place
                        let ref_ty = ctx.mir.ty_ref_mut(for_type);
                        let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                        let self_ref_place = Place::local(self_ref_local);

                        // Emit: %self_ref = ref var %literal
                        ctx.emit_assign(
                            self_ref_place.clone(),
                            Rvalue::RefMut(literal_place.clone()),
                        );

                        // Build call args: self_ref first (MutRef), then the i64 literal value (Borrow)
                        let call_args = vec![
                            CallArg::mutating(Value::Place(self_ref_place)),
                            CallArg::borrow(Value::Immediate(Immediate::i64(*value))),
                        ];

                        // Create a temp for the unit return value of init
                        let unit_ty = ctx.mir.ty_unit();
                        let unit_local = ctx.create_temp("init_ret", unit_ty);
                        let unit_place = Place::local(unit_local);

                        // Call the init function
                        let mir_callee = Callee::direct(init_name);
                        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                    } else {
                        // No init found - fall back to direct assignment (shouldn't happen for Matchable types)
                        ctx.emit_assign(literal_place.clone(), Rvalue::Use(Immediate::i64(*value)));
                    }
                } else {
                    // Not a struct type - fall back to direct assignment
                    ctx.emit_assign(literal_place.clone(), Rvalue::Use(Immediate::i64(*value)));
                }

                // Call Matchable.matches(switch_place, literal_place)
                // The matches method signature is: func matches(self, other: Self) -> Bool
                // Both parameters are passed by borrow
                let result_local = ctx.create_temp("matches_result", bool_mir_ty);
                let result_place_local = Place::local(result_local);

                let callee = Callee::witness(protocol_name, "matches", for_type);
                let call_args = vec![
                    CallArg::borrow(Value::Place(switch_place.clone())),
                    CallArg::borrow(Value::Place(literal_place)),
                ];

                ctx.emit_call_with_modes(result_place_local.clone(), callee, call_args);

                // Branch based on the bool result
                ctx.emit_branch(Value::Place(result_place_local), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::True | Constructor::False => {
                let bool_value = matches!(ctor, Constructor::True);

                // Create blocks for this case
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Create a temporary to hold the literal value
                // The type must conform to ExpressibleByBoolLiteral
                let literal_local = ctx.create_temp("literal", for_type);
                let literal_place = Place::local(literal_local);

                // Initialize the struct from the literal using the ExpressibleByBoolLiteral init
                if let TyKind::Struct {
                    symbol: struct_symbol,
                    ..
                } = ty.kind()
                {
                    // Find the init with the boolLiteral label
                    let init_symbol =
                        struct_symbol
                            .metadata()
                            .children()
                            .into_iter()
                            .find(|child| {
                                if child.metadata().kind() != KestrelSymbolKind::Initializer {
                                    return false;
                                }
                                // Check if this init has a parameter with the boolLiteral label
                                if let Some(callable) =
                                    child.metadata().get_behavior::<CallableBehavior>()
                                {
                                    callable.parameters().first().is_some_and(|p| {
                                        p.label.as_ref().is_some_and(|l| l.value == "boolLiteral")
                                    })
                                } else {
                                    false
                                }
                            });

                    if let Some(init_sym) = init_symbol {
                        // Build the qualified name for the init function
                        let mut name_parts = Vec::new();
                        collect_symbol_name_parts(
                            &(struct_symbol.clone()
                                as std::sync::Arc<
                                    dyn semantic_tree::symbol::Symbol<
                                            kestrel_semantic_tree::language::KestrelLanguage,
                                        >,
                                >),
                            &mut name_parts,
                        );

                        // Get all labels from the found init symbol
                        let init_name_suffix = if let Some(callable) =
                            init_sym.metadata().get_behavior::<CallableBehavior>()
                        {
                            let labels: Vec<&str> = callable
                                .parameters()
                                .iter()
                                .filter_map(|p| p.external_label())
                                .collect();
                            if labels.is_empty() {
                                "init".to_string()
                            } else {
                                format!("init${}", labels.join("$"))
                            }
                        } else {
                            "init$boolLiteral".to_string()
                        };
                        name_parts.push(init_name_suffix);

                        let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

                        // Create a mutable reference to the result place
                        let ref_ty = ctx.mir.ty_ref_mut(for_type);
                        let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                        let self_ref_place = Place::local(self_ref_local);

                        // Emit: %self_ref = ref var %literal
                        ctx.emit_assign(
                            self_ref_place.clone(),
                            Rvalue::RefMut(literal_place.clone()),
                        );

                        // Build call args: self_ref first (MutRef), then the i1 literal value (Borrow)
                        let call_args = vec![
                            CallArg::mutating(Value::Place(self_ref_place)),
                            CallArg::borrow(Value::Immediate(Immediate::bool(bool_value))),
                        ];

                        // Create a temp for the unit return value of init
                        let unit_ty = ctx.mir.ty_unit();
                        let unit_local = ctx.create_temp("init_ret", unit_ty);
                        let unit_place = Place::local(unit_local);

                        // Call the init function
                        let mir_callee = Callee::direct(init_name);
                        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                    } else {
                        // No init found - fall back to direct assignment (shouldn't happen for Matchable types)
                        ctx.emit_assign(
                            literal_place.clone(),
                            Rvalue::Use(Immediate::bool(bool_value)),
                        );
                    }
                } else {
                    // Not a struct type - fall back to direct assignment
                    ctx.emit_assign(
                        literal_place.clone(),
                        Rvalue::Use(Immediate::bool(bool_value)),
                    );
                }

                // Call Matchable.matches(switch_place, literal_place)
                let result_local = ctx.create_temp("matches_result", bool_mir_ty);
                let result_place_local = Place::local(result_local);

                let callee = Callee::witness(protocol_name, "matches", for_type);
                let call_args = vec![
                    CallArg::borrow(Value::Place(switch_place.clone())),
                    CallArg::borrow(Value::Place(literal_place)),
                ];

                ctx.emit_call_with_modes(result_place_local.clone(), callee, call_args);

                // Branch based on the bool result
                ctx.emit_branch(Value::Place(result_place_local), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::StringLiteral(value) => {
                ctx.emit_error(LoweringError::internal(
                    format!(
                        "Matchable switch with string literal '{}' not yet fully implemented",
                        value
                    ),
                    None,
                ));
                continue;
            },

            Constructor::CharLiteral(c) => {
                // Create blocks for this case
                let match_block = ctx.create_block();
                let next_block = if is_last && default.is_none() {
                    ctx.create_block()
                } else {
                    ctx.create_block()
                };

                // Create a temporary to hold the literal value
                // The type must conform to ExpressibleByCharLiteral
                let literal_local = ctx.create_temp("literal", for_type);
                let literal_place = Place::local(literal_local);

                // Initialize the struct from the literal using the ExpressibleByCharLiteral init
                if let TyKind::Struct {
                    symbol: struct_symbol,
                    ..
                } = ty.kind()
                {
                    // Find the init$charLiteral method
                    let init_sym = struct_symbol
                        .metadata()
                        .children()
                        .into_iter()
                        .find(|child| {
                            child.metadata().kind() == KestrelSymbolKind::Initializer
                                && child
                                    .metadata()
                                    .get_behavior::<CallableBehavior>()
                                    .map(|c| {
                                        c.parameters().first().and_then(|p| p.external_label())
                                            == Some("charLiteral")
                                    })
                                    .unwrap_or(false)
                        });

                    if let Some(init_sym) = init_sym {
                        // Build the qualified name for the init function
                        let mut name_parts = Vec::new();
                        collect_symbol_name_parts(
                            &(struct_symbol.clone()
                                as std::sync::Arc<
                                    dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
                                >),
                            &mut name_parts,
                        );

                        // Get all labels from the found init symbol
                        let init_name_suffix = if let Some(callable) =
                            init_sym.metadata().get_behavior::<CallableBehavior>()
                        {
                            let labels: Vec<&str> = callable
                                .parameters()
                                .iter()
                                .filter_map(|p| p.external_label())
                                .collect();
                            if labels.is_empty() {
                                "init".to_string()
                            } else {
                                format!("init${}", labels.join("$"))
                            }
                        } else {
                            "init$charLiteral".to_string()
                        };
                        name_parts.push(init_name_suffix);

                        let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

                        // Create a mutable reference to the result place
                        let ref_ty = ctx.mir.ty_ref_mut(for_type);
                        let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                        let self_ref_place = Place::local(self_ref_local);

                        // Emit: %self_ref = ref var %literal
                        ctx.emit_assign(
                            self_ref_place.clone(),
                            Rvalue::RefMut(literal_place.clone()),
                        );

                        // Build call args: self_ref first (MutRef), then the i32 char literal value (Borrow)
                        let call_args = vec![
                            CallArg::mutating(Value::Place(self_ref_place)),
                            CallArg::borrow(Value::Immediate(Immediate::i32(*c as i32))),
                        ];

                        // Create a temp for the unit return value of init
                        let unit_ty = ctx.mir.ty_unit();
                        let unit_local = ctx.create_temp("init_ret", unit_ty);
                        let unit_place = Place::local(unit_local);

                        // Call the init function
                        let mir_callee = Callee::direct(init_name);
                        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                    } else {
                        // No init found - fall back to direct assignment (shouldn't happen for Matchable types)
                        ctx.emit_assign(
                            literal_place.clone(),
                            Rvalue::Use(Immediate::i32(*c as i32)),
                        );
                    }
                } else {
                    // Not a struct type - fall back to direct assignment
                    ctx.emit_assign(
                        literal_place.clone(),
                        Rvalue::Use(Immediate::i32(*c as i32)),
                    );
                }

                // Call Matchable.matches(switch_place, literal_place)
                let result_local = ctx.create_temp("matches_result", bool_mir_ty);
                let result_place_local = Place::local(result_local);

                let callee = Callee::witness(protocol_name, "matches", for_type);
                let call_args = vec![
                    CallArg::borrow(Value::Place(switch_place.clone())),
                    CallArg::borrow(Value::Place(literal_place)),
                ];

                ctx.emit_call_with_modes(result_place_local.clone(), callee, call_args);

                // Branch based on the bool result
                ctx.emit_branch(Value::Place(result_place_local), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            Constructor::CharRange { start, end } => {
                // Char ranges for Matchable types - create temporaries for the low and high bounds
                // and check if scrutinee is in range using LessOrEqual.lessThanOrEqual
                let match_block = ctx.create_block();
                let check_hi_block = ctx.create_block(); // Block for checking upper bound
                let next_block = ctx.create_block();

                // Helper to create a char literal temp
                let create_char_temp = |ctx: &mut LoweringContext, c: char, name: &str| -> Place {
                    let char_local = ctx.create_temp(name, for_type);
                    let char_place = Place::local(char_local);

                    if let TyKind::Struct {
                        symbol: struct_symbol,
                        ..
                    } = ty.kind()
                    {
                        // Find the init$charLiteral method
                        let init_sym =
                            struct_symbol
                                .metadata()
                                .children()
                                .into_iter()
                                .find(|child| {
                                    child.metadata().kind() == KestrelSymbolKind::Initializer
                                        && child
                                            .metadata()
                                            .get_behavior::<CallableBehavior>()
                                            .map(|c| {
                                                c.parameters()
                                                    .first()
                                                    .and_then(|p| p.external_label())
                                                    == Some("charLiteral")
                                            })
                                            .unwrap_or(false)
                                });

                        if let Some(init_sym) = init_sym {
                            let mut name_parts = Vec::new();
                            collect_symbol_name_parts(
                                &(struct_symbol.clone()
                                    as std::sync::Arc<
                                        dyn Symbol<
                                            kestrel_semantic_tree::language::KestrelLanguage,
                                        >,
                                    >),
                                &mut name_parts,
                            );

                            let init_name_suffix = if let Some(callable) =
                                init_sym.metadata().get_behavior::<CallableBehavior>()
                            {
                                let labels: Vec<&str> = callable
                                    .parameters()
                                    .iter()
                                    .filter_map(|p| p.external_label())
                                    .collect();
                                if labels.is_empty() {
                                    "init".to_string()
                                } else {
                                    format!("init${}", labels.join("$"))
                                }
                            } else {
                                "init$charLiteral".to_string()
                            };
                            name_parts.push(init_name_suffix);

                            let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

                            let ref_ty = ctx.mir.ty_ref_mut(for_type);
                            let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                            let self_ref_place = Place::local(self_ref_local);

                            ctx.emit_assign(
                                self_ref_place.clone(),
                                Rvalue::RefMut(char_place.clone()),
                            );

                            let call_args = vec![
                                CallArg::mutating(Value::Place(self_ref_place)),
                                CallArg::borrow(Value::Immediate(Immediate::i32(c as i32))),
                            ];

                            let unit_ty = ctx.mir.ty_unit();
                            let unit_local = ctx.create_temp("init_ret", unit_ty);
                            let unit_place = Place::local(unit_local);

                            let mir_callee = Callee::direct(init_name);
                            ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                        }
                    }

                    char_place
                };

                // Create start and end char temps
                let start_place = create_char_temp(ctx, *start, "range_start");
                let end_place = create_char_temp(ctx, *end, "range_end");

                // Get the LessOrEqual protocol for witness calls
                let less_or_equal_protocol_name =
                    ctx.mir.intern_name(QualifiedNameData::new(vec![
                        "std".to_string(),
                        "core".to_string(),
                        "LessOrEqual".to_string(),
                    ]));

                // Check: start <= scrutinee (start.lessThanOrEqual(scrutinee))
                let cmp1_local = ctx.create_temp("cmp_lo", bool_mir_ty);
                let cmp1_place = Place::local(cmp1_local);

                let callee1 =
                    Callee::witness(less_or_equal_protocol_name, "lessThanOrEqual", for_type);
                let call_args1 = vec![
                    CallArg::borrow(Value::Place(start_place)),
                    CallArg::borrow(Value::Place(switch_place.clone())),
                ];
                ctx.emit_call_with_modes(cmp1_place.clone(), callee1, call_args1);

                // Branch: if start <= scrutinee, check upper bound; otherwise skip
                ctx.emit_branch(Value::Place(cmp1_place), check_hi_block, next_block);

                // In check_hi_block: Check scrutinee <= end
                ctx.set_current_block(check_hi_block);

                let cmp2_local = ctx.create_temp("cmp_hi", bool_mir_ty);
                let cmp2_place = Place::local(cmp2_local);

                let callee2 =
                    Callee::witness(less_or_equal_protocol_name, "lessThanOrEqual", for_type);
                let call_args2 = vec![
                    CallArg::borrow(Value::Place(switch_place.clone())),
                    CallArg::borrow(Value::Place(end_place)),
                ];
                ctx.emit_call_with_modes(cmp2_place.clone(), callee2, call_args2);

                // Branch: if scrutinee <= end, go to match; otherwise skip
                ctx.emit_branch(Value::Place(cmp2_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);

                // Continue with next comparison
                ctx.set_current_block(next_block);
            },

            _ => {
                // For non-literal constructors, fall back to default handling
                if is_last {
                    emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
                    return;
                }
                continue;
            },
        }
    }

    // After all cases, emit default
    if let Some(default_tree) = default {
        emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
    } else {
        ctx.emit_unreachable();
    }
}
