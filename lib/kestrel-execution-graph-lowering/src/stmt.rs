//! Statement lowering - converts semantic statements to MIR.

use kestrel_execution_graph::{Immediate, Place, Rvalue, StatementKind as MirStatementKind, Value};
use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::{Expression, IfCondition};
use kestrel_semantic_tree::pattern::PatternKind;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};

use crate::context::LoweringContext;
use crate::expr::lower_expression;
use crate::pattern::lower_pattern;
use crate::ty::lower_type;

/// Lower a statement to MIR.
///
/// This handles let/var bindings, expression statements, and guard-let.
/// The generated MIR statements are added to the current block.
/// Temporary values are tracked and deinited at statement end.
pub fn lower_statement(ctx: &mut LoweringContext, stmt: &Statement) {
    // Don't process if block is already terminated
    if ctx.is_block_terminated() {
        return;
    }

    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            // Lower the value expression first
            if let Some(init_expr) = value {
                let init_value = lower_expression(ctx, init_expr);

                // If block got terminated during expression lowering (e.g., return),
                // don't try to do the pattern binding
                if !ctx.is_block_terminated() {
                    lower_pattern(ctx, pattern, init_value);

                    // Track the local for deinit if needed
                    if let PatternKind::Local { local_id, .. } = &pattern.kind {
                        let mir_local = ctx.get_local_unwrap(*local_id);
                        let needs_deinit = ctx.type_needs_deinit(&pattern.ty);
                        // Pass the semantic type for field drop order expansion
                        let ty = if needs_deinit {
                            Some(pattern.ty.clone())
                        } else {
                            None
                        };
                        ctx.track_local(mir_local, needs_deinit, ty);
                    }
                }
            } else {
                // No initializer - the local is created but uninitialized
                // In MIR, we don't need to do anything here - the local was
                // already created during function setup
                //
                // Still track it for scope purposes, but it doesn't need deinit
                // since it's uninitialized
                if let PatternKind::Local { local_id, .. } = &pattern.kind {
                    let mir_local = ctx.get_local_unwrap(*local_id);
                    ctx.track_local(mir_local, false, None);
                }
            }

            // Emit deinits for temporaries created during this statement
            ctx.emit_temp_deinits();
        }

        StatementKind::Expr(expr) => {
            // Lower the expression for its side effects
            // The result is discarded
            let _value = lower_expression(ctx, expr);

            // Emit deinits for temporaries created during this statement
            ctx.emit_temp_deinits();
        }

        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            lower_guard_let(ctx, conditions, else_block);
            // Note: temp deinits are handled within guard_let due to control flow
        }

        StatementKind::Deinit { local_id, .. } => {
            // Deinit statement explicitly runs the destructor for a variable.
            // This should call the type's deinit function if it has one.
            let mir_local = ctx.get_local_unwrap(*local_id);

            // Get the type for proper deinit expansion
            let ty = ctx.get_local_type(mir_local).cloned();

            // Emit the deinit - this will call deinit functions and drop fields properly
            let place = Place::local(mir_local);
            ctx.emit_deinit_for_place(&place, ty.as_ref());

            // Mark as moved so scope exit doesn't double-deinit
            ctx.mark_moved(mir_local);

            // Emit deinits for temporaries (though unlikely for `deinit x;`)
            ctx.emit_temp_deinits();
        }
    }
}

/// Lower a guard-let statement.
///
/// Guard-let is like an inverted if-let: if the pattern matches, execution
/// continues after the guard statement with bindings in scope. If the pattern
/// doesn't match, the else block executes (which must diverge).
///
/// ```text
/// guard let .Some(x) = opt else {
///     return 0
/// }
/// // x is in scope here
/// x * 2
/// ```
///
/// # MIR Structure
///
/// ```text
/// entry_block:
///     <evaluate conditions>
///     branch to success_block or else_block
///
/// else_block:
///     <lower else block statements>
///     // must diverge (return, break, continue)
///
/// success_block:
///     <bindings are in scope>
///     <continue with statements after guard>
/// ```
fn lower_guard_let(
    ctx: &mut LoweringContext,
    conditions: &[IfCondition],
    else_block: &CodeBlock,
) {
    // Create blocks
    let success_block = ctx.create_block();
    let else_block_id = ctx.create_block();

    // Lower the condition chain - this will emit pattern tests and bindings
    // Success → jump to success_block (with bindings emitted before the jump)
    // Failure → jump to else_block_id
    lower_guard_condition_chain(ctx, conditions, 0, success_block, else_block_id);

    // Lower the else block (must diverge - return/break/continue)
    ctx.set_current_block(else_block_id);
    for stmt in &else_block.statements {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // If there's a yield expression in the else block, lower it
    // (though for guard-let this should be rare since else must diverge)
    if !ctx.is_block_terminated() {
        if let Some(yield_expr) = &else_block.yield_expr {
            let _value = lower_expression(ctx, yield_expr);
        }
    }

    // The else block should have diverged, but if it didn't (which would be
    // a semantic error caught earlier), we need to handle it gracefully
    if !ctx.is_block_terminated() {
        // This shouldn't happen if semantic analysis caught it,
        // but emit unreachable as a safety measure
        ctx.emit_unreachable();
    }

    // Continue with success block - bindings are already in scope
    ctx.set_current_block(success_block);
}

/// Lower a chain of conditions for guard-let.
///
/// Similar to if-let condition chains, but:
/// - Success means pattern matched → continue to success_block
/// - Failure means pattern didn't match → jump to else_block
///
/// Bindings are emitted as we go, so they're visible in later conditions
/// and after the guard statement.
fn lower_guard_condition_chain(
    ctx: &mut LoweringContext,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Base case: all conditions processed, jump to success block
    if index >= conditions.len() {
        ctx.emit_jump(success_block);
        return;
    }

    match &conditions[index] {
        IfCondition::Expr(condition_expr) => {
            // Boolean condition: emit branch
            let cond_value = lower_expression(ctx, condition_expr);

            // If this is the last condition, branch directly to success/else
            if index == conditions.len() - 1 {
                ctx.emit_branch(cond_value, success_block, else_block);
            } else {
                // More conditions to check: create a block for the next condition
                let next_block = ctx.create_block();
                ctx.emit_branch(cond_value, next_block, else_block);
                ctx.set_current_block(next_block);
                lower_guard_condition_chain(ctx, conditions, index + 1, success_block, else_block);
            }
        }

        IfCondition::Let { pattern, value, .. } => {
            // Guard-let condition: use pattern matching
            lower_guard_let_condition(
                ctx,
                pattern,
                value,
                conditions,
                index,
                success_block,
                else_block,
            );
        }
    }
}

/// Lower a single guard-let condition using decision tree compilation.
fn lower_guard_let_condition(
    ctx: &mut LoweringContext,
    pattern: &kestrel_semantic_tree::pattern::Pattern,
    scrutinee_expr: &Expression,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::compile;

    // Lower the scrutinee
    let scrutinee_value = lower_expression(ctx, scrutinee_expr);

    // We need the scrutinee in a place for pattern matching
    let scrutinee_place = match scrutinee_value {
        Value::Place(p) => p,
        Value::Immediate(imm) => {
            // Store the immediate in a temporary
            let scrutinee_ty = lower_type(ctx, &scrutinee_expr.ty);
            let scrutinee_local = ctx.create_temp("scrutinee", scrutinee_ty);
            let place = Place::local(scrutinee_local);
            ctx.emit_assign(place.clone(), Rvalue::Use(imm));
            place
        }
    };

    // Compile the pattern into a decision tree
    let patterns = vec![pattern.clone()];
    let has_guards = vec![false];
    let decision_tree = compile(&patterns, &scrutinee_expr.ty, &has_guards);

    // Emit the decision tree for guard-let
    emit_guard_let_decision_tree(
        ctx,
        &decision_tree,
        &scrutinee_place,
        conditions,
        index,
        success_block,
        else_block,
    );
}

/// Emit MIR for a guard-let decision tree.
///
/// Similar to if-let, but:
/// - Success: emit bindings and continue to next condition (or success_block)
/// - Failure: jump to else_block
fn emit_guard_let_decision_tree(
    ctx: &mut LoweringContext,
    tree: &kestrel_semantic_pattern_matching::DecisionTree,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::DecisionTree;

    match tree {
        DecisionTree::Success { bindings, .. } => {
            // Pattern matched! Emit bindings and continue to next condition
            crate::match_lowering::emit_bindings(ctx, bindings, scrutinee);

            // Continue with the rest of the condition chain
            lower_guard_condition_chain(ctx, conditions, index + 1, success_block, else_block);
        }

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_guard_let_switch(
                ctx,
                path,
                ty,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        }

        DecisionTree::Guard { .. } => {
            // Guards shouldn't appear in guard-let patterns
            ctx.emit_jump(else_block);
        }

        DecisionTree::Failure => {
            // Pattern didn't match, go to else block
            ctx.emit_jump(else_block);
        }
    }
}

/// Emit MIR for a switch node in a guard-let decision tree.
fn emit_guard_let_switch(
    ctx: &mut LoweringContext,
    path: &kestrel_semantic_pattern_matching::AccessPath,
    ty: &kestrel_semantic_tree::ty::Ty,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_tree::ty::TyKind;

    // Get the place to switch on
    let switch_place = crate::match_lowering::apply_path(scrutinee, path);

    match ty.kind() {
        TyKind::Bool => {
            emit_guard_let_bool_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        }

        TyKind::Enum { .. } => {
            emit_guard_let_enum_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        }

        TyKind::Int(_) => {
            emit_guard_let_int_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        }

        TyKind::String => {
            emit_guard_let_string_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        }

        TyKind::Tuple(_) | TyKind::Struct { .. } => {
            // Single constructor types - just recurse into the case
            if let Some((_, subtree)) = cases.first() {
                emit_guard_let_decision_tree(
                    ctx,
                    subtree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );
            } else if let Some(default_tree) = default {
                emit_guard_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );
            } else {
                ctx.emit_jump(else_block);
            }
        }

        _ => {
            // For other types, try the default or first case
            if let Some(default_tree) = default {
                emit_guard_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );
            } else if let Some((_, tree)) = cases.first() {
                emit_guard_let_decision_tree(
                    ctx,
                    tree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );
            } else {
                ctx.emit_jump(else_block);
            }
        }
    }
}

/// Emit boolean switch for guard-let.
fn emit_guard_let_bool_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

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
        emit_guard_let_decision_tree(
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else if let Some(default_tree) = default {
        emit_guard_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }

    // Emit false case
    ctx.set_current_block(false_block);
    if let Some(tree) = false_tree {
        emit_guard_let_decision_tree(
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else if let Some(default_tree) = default {
        emit_guard_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit enum switch for guard-let.
fn emit_guard_let_enum_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Build switch cases: (variant_name, block)
    let mut switch_cases = Vec::with_capacity(cases.len() + 1);
    let mut case_trees = Vec::with_capacity(cases.len());

    for (ctor, tree) in cases {
        if let Constructor::Variant { name, .. } = ctor {
            let case_block = ctx.create_block();
            switch_cases.push((name.clone(), case_block));
            case_trees.push((case_block, tree));
        }
    }

    // Add default case (for unmatched variants → else block)
    let default_case_block = ctx.create_block();
    switch_cases.push(("_".to_string(), default_case_block));

    // Emit the switch terminator
    ctx.emit_switch(switch_place.clone(), switch_cases);

    // Emit each matched variant's body
    for (block, tree) in case_trees {
        ctx.set_current_block(block);
        emit_guard_let_decision_tree(
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    }

    // Emit default case: if there's a default tree use it, otherwise go to else
    ctx.set_current_block(default_case_block);
    if let Some(default_tree) = default {
        emit_guard_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit integer comparison chain for guard-let.
fn emit_guard_let_int_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_guard_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        } else {
            ctx.emit_jump(else_block);
        }
        return;
    }

    // Build a chain of comparisons
    for (ctor, tree) in cases.iter() {
        match ctor {
            Constructor::IntLiteral(value) => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

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

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_guard_let_decision_tree(
                    ctx,
                    tree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );

                // Continue with next comparison
                ctx.set_current_block(next_block);
            }

            Constructor::IntRange { start, end } => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Range check: start <= value && value <= end
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

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                ctx.set_current_block(match_block);
                emit_guard_let_decision_tree(
                    ctx,
                    tree,
                    scrutinee,
                    conditions,
                    index,
                    success_block,
                    else_block,
                );

                ctx.set_current_block(next_block);
            }

            _ => {
                // Skip unsupported constructors
                continue;
            }
        }
    }

    // After all cases, check default or go to else
    if let Some(default_tree) = default {
        emit_guard_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit string comparison chain for guard-let.
fn emit_guard_let_string_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    success_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_guard_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );
        } else {
            ctx.emit_jump(else_block);
        }
        return;
    }

    // Build a chain of string comparisons
    for (ctor, tree) in cases {
        if let Constructor::StringLiteral(value) = ctor {
            let match_block = ctx.create_block();
            let next_block = ctx.create_block();

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

            ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

            ctx.set_current_block(match_block);
            emit_guard_let_decision_tree(
                ctx,
                tree,
                scrutinee,
                conditions,
                index,
                success_block,
                else_block,
            );

            ctx.set_current_block(next_block);
        }
    }

    // After all cases, check default or go to else
    if let Some(default_tree) = default {
        emit_guard_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            success_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}
