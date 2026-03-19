//! # Definite Assignment Analyzer
//!
//! Checks that all local variables are assigned before use. Tracks a
//! `HashSet<LocalId>` of definitely-assigned variables through control flow,
//! merging branches conservatively (intersection for if/else and match).
//!
//! ## Diagnostics
//!
//! ### KS004 — `uninitialized_variable_access` (Error, Correctness)
//!
//! **Message:** "access to uninitialized variable '{name}'"
//!
//! **Labels:**
//! - Primary: the expression referencing the uninitialized local
//!   - Span source: `util::expr_span` on the `HirExprId` containing `HirExpr::Local`
//!   - Message: "variable not initialized"
//!
//! **Notes:** (none)

use std::collections::HashSet;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS004",
    name: "uninitialized_variable_access",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DefiniteAssignmentAnalyzer;

impl Describe for DefiniteAssignmentAnalyzer {
    fn id(&self) -> &'static str {
        "definite_assignment"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for DefiniteAssignmentAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Start with all parameters marked as assigned
        let mut assigned: HashSet<LocalId> = cx.hir.params.iter().copied().collect();
        let mut diags = Vec::new();

        analyze_block(cx, &cx.hir.statements, cx.hir.tail_expr, &mut assigned, &mut diags);
        diags
    }
}

// ===== Dataflow state =====

#[derive(Clone, Debug)]
struct State {
    assigned: HashSet<LocalId>,
    diverged: bool,
}

// ===== Block / statement / expression analysis =====

/// Analyze a block, tracking which locals are assigned. Returns the state
/// after the block (including whether it diverges).
fn analyze_block(
    cx: &BodyContext<'_>,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    assigned: &mut HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    let mut state = State {
        assigned: assigned.clone(),
        diverged: false,
    };

    for &stmt_id in stmts {
        if state.diverged {
            break;
        }
        state = analyze_stmt(cx, stmt_id, state, diags);
    }

    if !state.diverged {
        if let Some(tail) = tail {
            state = analyze_expr(cx, tail, state, false, diags);
        }
    }

    // Propagate assignments back to the caller
    *assigned = state.assigned.clone();
    state
}

fn analyze_stmt(
    cx: &BodyContext<'_>,
    id: HirStmtId,
    mut state: State,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    match &cx.hir.stmts[id] {
        HirStmt::Let { local, value, .. } => {
            // Analyze the value expression first (before marking the local as assigned)
            if let Some(val) = value {
                state = analyze_expr(cx, *val, state, false, diags);
                // Mark the local as assigned after the value is evaluated
                state.assigned.insert(*local);
                // Also mark any pattern bindings if this is a destructuring let
                // (In HIR, let bindings are flat — each gets its own local)
            }
            // If no value, the local stays unassigned (e.g. `let x: Int`)
        }
        HirStmt::Expr { expr, .. } => {
            state = analyze_expr(cx, *expr, state, false, diags);
        }
        HirStmt::Deinit { .. } => {
            // Deinit doesn't affect assignment state
        }
    }
    state
}

fn analyze_expr(
    cx: &BodyContext<'_>,
    id: HirExprId,
    mut state: State,
    is_assign_target: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    match &cx.hir.exprs[id] {
        // Reading a local: check it's assigned (unless this is an assignment target)
        HirExpr::Local(local_id, _) => {
            if !is_assign_target && !state.assigned.contains(local_id) {
                let name = cx.hir.locals[*local_id].name.clone();
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("access to uninitialized variable '{}'", name),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, id),
                        message: "variable not initialized".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        // Assignment: analyze value first, then mark target local as assigned
        HirExpr::Assign { target, value, .. } => {
            state = analyze_expr(cx, *value, state, false, diags);
            // If target is a local, mark it assigned
            if let HirExpr::Local(local_id, _) = &cx.hir.exprs[*target] {
                state.assigned.insert(*local_id);
            }
            state = analyze_expr(cx, *target, state, true, diags);
        }

        // If/else: merge assigned sets from both branches (intersection)
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            state = analyze_expr(cx, *condition, state, false, diags);
            let pre_if = state.assigned.clone();

            // Then branch
            let mut then_assigned = pre_if.clone();
            let then_state = analyze_block(
                cx,
                &then_body.stmts,
                then_body.tail_expr,
                &mut then_assigned,
                diags,
            );

            // Else branch
            let else_state = if let Some(else_block) = else_body {
                let mut else_assigned = pre_if.clone();
                let es = analyze_block(
                    cx,
                    &else_block.stmts,
                    else_block.tail_expr,
                    &mut else_assigned,
                    diags,
                );
                (es, else_assigned)
            } else {
                (
                    State {
                        assigned: pre_if.clone(),
                        diverged: false,
                    },
                    pre_if,
                )
            };

            // Merge: if both diverge, take intersection. If one diverges, take the other.
            if then_state.diverged && else_state.0.diverged {
                state.diverged = true;
                state.assigned = then_state
                    .assigned
                    .intersection(&else_state.0.assigned)
                    .copied()
                    .collect();
            } else if then_state.diverged {
                state = else_state.0;
                state.assigned = else_state.1;
            } else if else_state.0.diverged {
                state = then_state;
                state.assigned = then_assigned;
            } else {
                // Neither diverges: intersection of both branches
                state.assigned = then_assigned
                    .intersection(&else_state.1)
                    .copied()
                    .collect();
            }
        }

        // Match: intersection of all arm states
        HirExpr::Match { scrutinee, arms, .. } => {
            state = analyze_expr(cx, *scrutinee, state, false, diags);

            if arms.is_empty() {
                return state;
            }

            let mut arm_states = Vec::new();
            for arm in arms {
                let mut arm_state = state.clone();
                // Pattern bindings are assigned within the arm
                mark_pattern_assigned(cx.hir, arm.pattern, &mut arm_state.assigned);
                if let Some(guard) = arm.guard {
                    arm_state = analyze_expr(cx, guard, arm_state, false, diags);
                }
                arm_state = analyze_expr(cx, arm.body, arm_state, false, diags);
                arm_states.push(arm_state);
            }

            // Merge all arms: if all diverge, result diverges.
            // Assigned = intersection of all non-diverging arms (or all arms if all diverge).
            let all_diverge = arm_states.iter().all(|s| s.diverged);
            let mut merged_assigned: Option<HashSet<LocalId>> = None;
            for arm_state in &arm_states {
                match &mut merged_assigned {
                    None => merged_assigned = Some(arm_state.assigned.clone()),
                    Some(set) => {
                        *set = set.intersection(&arm_state.assigned).copied().collect();
                    }
                }
            }
            state.diverged = all_diverge;
            if let Some(merged) = merged_assigned {
                state.assigned = merged;
            }
        }

        // Loop: body may not execute (well, it always does at least once, but
        // we conservatively don't trust loop body assignments since break can
        // exit before assignments happen). Analyze body for errors though.
        HirExpr::Loop { body, .. } => {
            let mut body_assigned = state.assigned.clone();
            let body_state = analyze_block(
                cx,
                &body.stmts,
                body.tail_expr,
                &mut body_assigned,
                diags,
            );

            // If the body always returns (not via break), the loop diverges
            if body_state.diverged && !block_contains_break(cx.hir, body) {
                state.diverged = true;
            }
            // Don't merge body assignments — loop body might not fully execute
        }

        // Block expression
        HirExpr::Block { body, .. } => {
            let mut block_assigned = state.assigned.clone();
            let block_state = analyze_block(
                cx,
                &body.stmts,
                body.tail_expr,
                &mut block_assigned,
                diags,
            );
            state.assigned = block_assigned;
            state.diverged = block_state.diverged;
        }

        // Return / break / continue: diverges
        HirExpr::Return { value, .. } => {
            if let Some(val) = value {
                state = analyze_expr(cx, *val, state, false, diags);
            }
            state.diverged = true;
        }
        HirExpr::Break { .. } | HirExpr::Continue { .. } => {
            state.diverged = true;
        }

        // Closures: analyze body separately (captures are already assigned from outer scope).
        // Closure body doesn't affect outer assignment state.
        HirExpr::Closure { body, .. } => {
            let mut closure_assigned = state.assigned.clone();
            // Mark closure params as assigned
            // (params are part of the body's locals, referenced by index)
            let _ = analyze_block(
                cx,
                &body.stmts,
                body.tail_expr,
                &mut closure_assigned,
                diags,
            );
        }

        // Expressions that recurse into sub-expressions
        HirExpr::Call { callee, args, .. } => {
            state = analyze_expr(cx, *callee, state, false, diags);
            for arg in args {
                state = analyze_expr(cx, arg.value, state, false, diags);
            }
        }
        HirExpr::MethodCall {
            receiver, args, ..
        }
        | HirExpr::ProtocolCall {
            receiver, args, ..
        } => {
            state = analyze_expr(cx, *receiver, state, false, diags);
            for arg in args {
                state = analyze_expr(cx, arg.value, state, false, diags);
            }
        }
        HirExpr::Field { base, .. } => {
            state = analyze_expr(cx, *base, state, false, diags);
        }
        HirExpr::TupleIndex { base, .. } => {
            state = analyze_expr(cx, *base, state, false, diags);
        }
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &elem in elements {
                state = analyze_expr(cx, elem, state, false, diags);
            }
        }
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                state = analyze_expr(cx, entry.key, state, false, diags);
                state = analyze_expr(cx, entry.value, state, false, diags);
            }
        }
        HirExpr::ImplicitMember { args, .. } => {
            if let Some(args) = args {
                for arg in args {
                    state = analyze_expr(cx, arg.value, state, false, diags);
                }
            }
        }

        // Leaf expressions: no sub-expressions to check
        HirExpr::Literal { .. }
        | HirExpr::Def(..)
        | HirExpr::OverloadSet { .. }
        | HirExpr::Error { .. } => {}
    }

    state
}

// ===== Pattern assignment =====

/// Mark all locals bound by a pattern as assigned.
fn mark_pattern_assigned(hir: &HirBody, pat_id: HirPatId, assigned: &mut HashSet<LocalId>) {
    match &hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            assigned.insert(*local);
        }
        HirPat::Tuple { elements, .. } => {
            for &elem in elements {
                mark_pattern_assigned(hir, elem, assigned);
            }
        }
        HirPat::Variant { args, .. } | HirPat::ImplicitVariant { args, .. } => {
            for arg in args {
                mark_pattern_assigned(hir, arg.pattern, assigned);
            }
        }
        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(pat) = field.pattern {
                    mark_pattern_assigned(hir, pat, assigned);
                }
            }
        }
        HirPat::Or { alternatives, .. } => {
            // All alternatives must bind the same locals; take from first
            if let Some(&first) = alternatives.first() {
                mark_pattern_assigned(hir, first, assigned);
            }
        }
        HirPat::At {
            binding,
            subpattern,
            ..
        } => {
            assigned.insert(*binding);
            mark_pattern_assigned(hir, *subpattern, assigned);
        }
        HirPat::Wildcard { .. }
        | HirPat::Literal { .. }
        | HirPat::Range { .. }
        | HirPat::Error { .. } => {}
    }
}

// ===== Break detection for loop divergence =====

fn block_contains_break(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if stmt_contains_break(hir, stmt_id) {
            return true;
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_contains_break(hir, tail);
    }
    false
}

fn stmt_contains_break(hir: &HirBody, id: HirStmtId) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_contains_break(hir, *expr),
        HirStmt::Let { value: Some(v), .. } => expr_contains_break(hir, *v),
        _ => false,
    }
}

fn expr_contains_break(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Break { .. } => true,
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_contains_break(hir, then_body)
                || else_body.as_ref().is_some_and(|e| block_contains_break(hir, e))
        }
        HirExpr::Match { arms, .. } => arms.iter().any(|arm| expr_contains_break(hir, arm.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),
        // Don't recurse into nested loops or closures
        HirExpr::Loop { .. } | HirExpr::Closure { .. } => false,
        _ => false,
    }
}
