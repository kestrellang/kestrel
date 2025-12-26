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

use kestrel_execution_graph::{BinOp, Immediate, Place, Rvalue, Value};
use kestrel_semantic_pattern_matching::{
    compile, AccessPath, Binding, Constructor, DecisionTree, PathElement,
};
use kestrel_semantic_tree::expr::{Expression, MatchArm};
use kestrel_semantic_tree::ty::{Ty, TyKind};

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::ty::lower_type;

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
        }
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
        DecisionTree::Success { arm_index, bindings } => {
            emit_success(ctx, *arm_index, bindings, scrutinee, arms, result_place, join_block);
        }

        DecisionTree::Switch { path, ty, cases, default } => {
            emit_switch(ctx, path, ty, cases, default, scrutinee, arms, result_place, join_block);
        }

        DecisionTree::Guard { arm_index, bindings, success, failure } => {
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
        }

        DecisionTree::Failure => {
            // Unreachable - exhaustiveness checking should prevent this
            ctx.emit_unreachable();
        }
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

    // Assign result
    ctx.emit_assign_value(result_place.clone(), body_value);

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
            emit_bool_switch(ctx, &switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }

        TyKind::Enum { .. } => {
            emit_enum_switch(ctx, &switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }

        TyKind::Int(_) => {
            emit_int_switch(ctx, &switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }

        TyKind::String => {
            emit_string_switch(ctx, &switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }

        TyKind::Tuple(_) => {
            // Tuples have a single constructor - just recurse into the single case
            if let Some((_, subtree)) = cases.first() {
                emit_decision_tree(ctx, subtree, scrutinee, arms, result_place, join_block);
            } else if let Some(default_tree) = default {
                emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
            } else {
                ctx.emit_unreachable();
            }
        }

        TyKind::Struct { .. } => {
            // Structs have a single constructor - just recurse
            if let Some((_, subtree)) = cases.first() {
                emit_decision_tree(ctx, subtree, scrutinee, arms, result_place, join_block);
            } else if let Some(default_tree) = default {
                emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
            } else {
                ctx.emit_unreachable();
            }
        }

        _ => {
            // For other types, emit a comparison chain
            emit_comparison_chain(ctx, &switch_place, ty, cases, default, scrutinee, arms, result_place, join_block);
        }
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
    let true_tree = cases.iter().find(|(c, _)| matches!(c, Constructor::True)).map(|(_, t)| t);
    let false_tree = cases.iter().find(|(c, _)| matches!(c, Constructor::False)).map(|(_, t)| t);

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
    cases: &[(Constructor, DecisionTree)],
    default: &Option<Box<DecisionTree>>,
    scrutinee: &Place,
    arms: &[MatchArm],
    result_place: &Place,
    join_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    emit_comparison_chain_int(ctx, switch_place, cases, default, scrutinee, arms, result_place, join_block);
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
    emit_comparison_chain_string(ctx, switch_place, cases, default, scrutinee, arms, result_place, join_block);
}

/// Emit a comparison chain for integer literals.
fn emit_comparison_chain_int(
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
                        rhs: Value::Immediate(Immediate::i64(*value)),
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
                        lhs: Value::Immediate(Immediate::i64(*start)),
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
                        rhs: Value::Immediate(Immediate::i64(*end)),
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
            }

            _ => {
                // Unsupported constructor for int switch
                ctx.emit_error(LoweringError::internal(
                    format!("unsupported constructor in int switch: {:?}", ctor),
                    None,
                ));
                ctx.emit_unreachable();
                return;
            }
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
    for (i, (ctor, tree)) in cases.iter().enumerate() {
        if let Constructor::StringLiteral(value) = ctor {
            let is_last = i == cases.len() - 1;

            let match_block = ctx.create_block();
            let next_block = if is_last && default.is_none() {
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
        TyKind::Int(_) => {
            emit_comparison_chain_int(ctx, switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }
        TyKind::String => {
            emit_comparison_chain_string(ctx, switch_place, cases, default, scrutinee, arms, result_place, join_block);
        }
        _ => {
            // Fallback: just emit default or first case
            if let Some(default_tree) = default {
                emit_decision_tree(ctx, default_tree, scrutinee, arms, result_place, join_block);
            } else if let Some((_, tree)) = cases.first() {
                emit_decision_tree(ctx, tree, scrutinee, arms, result_place, join_block);
            } else {
                ctx.emit_unreachable();
            }
        }
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
    let guard_expr = arm.guard.as_ref().expect("Guard arm should have guard expression");
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
