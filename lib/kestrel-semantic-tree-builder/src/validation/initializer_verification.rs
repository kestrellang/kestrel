//! Validator for initializer verification
//!
//! This validator verifies that initializers correctly initialize all fields:
//! - All fields must be assigned before the initializer returns
//! - `let` fields can only be assigned once
//! - Fields cannot be read before they are assigned
//! - Methods cannot be called on `self` before all fields are initialized
//!
//! Control flow analysis:
//! - If/else: A field is definitely initialized only if initialized in BOTH branches
//!   (unless one branch diverges via return/break/continue)
//! - While loops: Body may execute zero times, so initializations inside are not guaranteed
//! - Loop: With break, we track what's initialized at each break point
//! - Return: Marks the current path as diverging; all fields must be initialized before return

use std::collections::HashSet;
use std::sync::Arc;

use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::database::SemanticDatabase;
use crate::validation::{SymbolContext, Validator};

/// Validator for initializer field initialization
pub struct InitializerVerificationValidator;

impl InitializerVerificationValidator {
    const NAME: &'static str = "initializer_verification";

    pub fn new() -> Self {
        Self
    }
}

impl Default for InitializerVerificationValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for InitializerVerificationValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check initializers
        if kind != KestrelSymbolKind::Initializer {
            return;
        }

        validate_initializer(ctx.symbol, ctx.db, &mut *ctx.diagnostics().get());
    }
}

/// Validate a single initializer
fn validate_initializer(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    _db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    // Get the parent struct to know what fields need to be initialized
    let Some(parent) = symbol.metadata().parent() else {
        return;
    };

    if parent.metadata().kind() != KestrelSymbolKind::Struct {
        return;
    }

    // Collect all fields that need to be initialized
    let fields: Vec<FieldInfo> = parent
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
        .map(|f| {
            let name = f.metadata().name().value.clone();
            let is_let = !is_field_mutable(&f);
            FieldInfo { name, is_let }
        })
        .collect();

    // Get the executable behavior (body)
    let Some(body) = get_executable_body(symbol) else {
        // No body - this is an error but handled elsewhere
        return;
    };

    // Create verification context with initial state
    let all_fields: HashSet<String> = fields.iter().map(|f| f.name.clone()).collect();
    let let_fields: HashSet<String> = fields
        .iter()
        .filter(|f| f.is_let)
        .map(|f| f.name.clone())
        .collect();

    let mut ctx = VerificationContext {
        all_fields: all_fields.clone(),
        let_fields,
        state: InitState::new(),
        errors: Vec::new(),
    };

    // Analyze the body
    let final_state = analyze_block(&body.statements, body.yield_expr.as_deref(), &mut ctx);

    // Check that all fields are initialized at the end (if we didn't diverge)
    if !final_state.diverged {
        let uninitialized: Vec<&String> = all_fields
            .iter()
            .filter(|f| !final_state.assigned.contains(*f))
            .collect();

        if !uninitialized.is_empty() {
            let file_id = crate::syntax::get_file_id_for_symbol(symbol, diagnostics);
            let span = symbol.metadata().span().clone();

            let field_list = uninitialized
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ");

            let error = UninitializedFieldsError {
                span,
                fields: field_list,
            };
            diagnostics.add_diagnostic(error.into_diagnostic(file_id));
        }
    }

    // Report any errors collected during analysis
    let file_id = crate::syntax::get_file_id_for_symbol(symbol, diagnostics);
    for error in ctx.errors {
        diagnostics.add_diagnostic(error.into_diagnostic(file_id));
    }
}

/// Information about a field
struct FieldInfo {
    name: String,
    is_let: bool,
}

/// Initialization state at a point in the program
#[derive(Clone, Debug)]
struct InitState {
    /// Fields that are definitely assigned at this point
    assigned: HashSet<String>,
    /// `let` fields that have been assigned (for double-assignment detection)
    let_assigned: HashSet<String>,
    /// Whether this path has diverged (return/break/continue)
    diverged: bool,
}

impl InitState {
    fn new() -> Self {
        Self {
            assigned: HashSet::new(),
            let_assigned: HashSet::new(),
            diverged: false,
        }
    }

    /// Merge two states from different branches (e.g., if/else)
    /// A field is definitely initialized only if initialized in BOTH branches,
    /// unless one branch diverged.
    fn merge(self, other: InitState) -> InitState {
        if self.diverged && other.diverged {
            // Both branches diverge - the merged state also diverges
            InitState {
                assigned: self.assigned.intersection(&other.assigned).cloned().collect(),
                let_assigned: self.let_assigned.union(&other.let_assigned).cloned().collect(),
                diverged: true,
            }
        } else if self.diverged {
            // Only self diverged - use other's state
            other
        } else if other.diverged {
            // Only other diverged - use self's state
            self
        } else {
            // Neither diverged - intersection of assigned fields
            InitState {
                assigned: self.assigned.intersection(&other.assigned).cloned().collect(),
                // Union of let_assigned to track all assignments for double-assign detection
                let_assigned: self.let_assigned.union(&other.let_assigned).cloned().collect(),
                diverged: false,
            }
        }
    }
}

/// Context for tracking field initialization state
struct VerificationContext {
    /// All field names that need to be initialized
    all_fields: HashSet<String>,
    /// Fields declared with `let` (can only be assigned once)
    let_fields: HashSet<String>,
    /// Current initialization state
    state: InitState,
    /// Collected errors
    errors: Vec<InitializerError>,
}

/// Errors that can occur during initializer verification
enum InitializerError {
    LetFieldAssignedTwice { span: Span, field_name: String },
    FieldReadBeforeAssigned { span: Span, field_name: String },
    SelfUsedBeforeFullyInitialized { span: Span, uninitialized: Vec<String> },
    ReturnBeforeFullyInitialized { span: Span, uninitialized: Vec<String> },
}

impl IntoDiagnostic for InitializerError {
    fn into_diagnostic(&self, file_id: usize) -> Diagnostic<usize> {
        match self {
            InitializerError::LetFieldAssignedTwice { span, field_name } => Diagnostic::error()
                .with_message(format!(
                    "cannot assign to 'let' field '{}' more than once",
                    field_name
                ))
                .with_labels(vec![
                    Label::primary(file_id, span.clone()).with_message("second assignment here")
                ]),
            InitializerError::FieldReadBeforeAssigned { span, field_name } => Diagnostic::error()
                .with_message(format!(
                    "cannot read field '{}' before it is initialized",
                    field_name
                ))
                .with_labels(vec![
                    Label::primary(file_id, span.clone()).with_message("field read here")
                ]),
            InitializerError::SelfUsedBeforeFullyInitialized { span, uninitialized } => {
                let fields = uninitialized.join(", ");
                Diagnostic::error()
                    .with_message("cannot use 'self' before all fields are initialized")
                    .with_labels(vec![
                        Label::primary(file_id, span.clone()).with_message("self used here")
                    ])
                    .with_notes(vec![format!("uninitialized fields: {}", fields)])
            }
            InitializerError::ReturnBeforeFullyInitialized { span, uninitialized } => {
                let fields = uninitialized.join(", ");
                Diagnostic::error()
                    .with_message("cannot return before all fields are initialized")
                    .with_labels(vec![
                        Label::primary(file_id, span.clone()).with_message("return here")
                    ])
                    .with_notes(vec![format!("uninitialized fields: {}", fields)])
            }
        }
    }
}

/// Error for uninitialized fields at end of initializer
struct UninitializedFieldsError {
    span: Span,
    fields: String,
}

impl IntoDiagnostic for UninitializedFieldsError {
    fn into_diagnostic(&self, file_id: usize) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "initializer does not initialize all fields: {}",
                self.fields
            ))
            .with_labels(vec![
                Label::primary(file_id, self.span.clone()).with_message("in this initializer")
            ])
    }
}

/// Analyze a block of statements and optional yield expression
/// Returns the initialization state after the block
fn analyze_block(
    statements: &[Statement],
    yield_expr: Option<&Expression>,
    ctx: &mut VerificationContext,
) -> InitState {
    let mut state = ctx.state.clone();

    for stmt in statements {
        if state.diverged {
            // Dead code after divergence - could warn but skip for now
            break;
        }
        state = analyze_statement(stmt, state, ctx);
    }

    // Analyze yield expression if present and not diverged
    if !state.diverged {
        if let Some(expr) = yield_expr {
            state = analyze_expression(expr, state, false, ctx);
        }
    }

    state
}

/// Analyze a statement, returning the new initialization state
fn analyze_statement(
    stmt: &Statement,
    mut state: InitState,
    ctx: &mut VerificationContext,
) -> InitState {
    match &stmt.kind {
        StatementKind::Binding { pattern: _, value } => {
            // Variable binding - analyze the value expression
            if let Some(expr) = value {
                state = analyze_expression(expr, state, false, ctx);
            }
        }
        StatementKind::Expr(expr) => {
            // Expression statement
            state = analyze_expression(expr, state, false, ctx);
        }
    }
    state
}

/// Analyze an expression, returning the new initialization state
///
/// `is_assignment_target` indicates if this expression is the LHS of an assignment.
fn analyze_expression(
    expr: &Expression,
    mut state: InitState,
    is_assignment_target: bool,
    ctx: &mut VerificationContext,
) -> InitState {
    match &expr.kind {
        ExprKind::FieldAccess { object, field } => {
            // Check if this is `self.field`
            if is_self_expr(object) {
                if !is_assignment_target {
                    // This is a read of self.field - must be initialized first
                    if !state.assigned.contains(field) {
                        ctx.errors.push(InitializerError::FieldReadBeforeAssigned {
                            span: expr.span.clone(),
                            field_name: field.clone(),
                        });
                    }
                }
                // If it's an assignment target, the assignment is handled in ExprKind::Assignment
            } else {
                // Analyze the object expression
                state = analyze_expression(object, state, false, ctx);
            }
        }
        ExprKind::Call { callee, arguments, .. } => {
            // Check if this is a method call on self
            if let ExprKind::MethodRef { receiver, .. } = &callee.kind {
                if is_self_expr(receiver) {
                    // Method call on self - all fields must be initialized
                    let uninitialized: Vec<String> = ctx
                        .all_fields
                        .iter()
                        .filter(|f| !state.assigned.contains(*f))
                        .cloned()
                        .collect();

                    if !uninitialized.is_empty() {
                        ctx.errors
                            .push(InitializerError::SelfUsedBeforeFullyInitialized {
                                span: expr.span.clone(),
                                uninitialized,
                            });
                    }
                }
            }

            // Analyze callee and arguments
            state = analyze_expression(callee, state, false, ctx);
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
        }
        ExprKind::LocalRef(_) => {}
        ExprKind::SymbolRef(_) => {}
        ExprKind::TypeRef(_) => {}
        ExprKind::TypeParameterRef(_) => {}
        ExprKind::OverloadedRef(_) => {}
        ExprKind::MethodRef { receiver, .. } => {
            state = analyze_expression(receiver, state, false, ctx);
        }
        ExprKind::TupleIndex { tuple, .. } => {
            state = analyze_expression(tuple, state, false, ctx);
        }
        ExprKind::Literal(_) => {}
        ExprKind::Array(elements) => {
            for elem in elements {
                state = analyze_expression(elem, state, false, ctx);
            }
        }
        ExprKind::Tuple(elements) => {
            for elem in elements {
                state = analyze_expression(elem, state, false, ctx);
            }
        }
        ExprKind::Grouping(inner) => {
            state = analyze_expression(inner, state, false, ctx);
        }
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            state = analyze_expression(receiver, state, false, ctx);
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
        }
        ExprKind::Assignment { target, value } => {
            // First analyze the value (RHS) - this happens before assignment
            state = analyze_expression(value, state, false, ctx);

            // Check if this is `self.field = value`
            if let ExprKind::FieldAccess { object, field } = &target.kind {
                if is_self_expr(object) {
                    // Check for double-assignment to `let` fields
                    if ctx.let_fields.contains(field) && state.let_assigned.contains(field) {
                        ctx.errors.push(InitializerError::LetFieldAssignedTwice {
                            span: target.span.clone(),
                            field_name: field.clone(),
                        });
                    }
                    // Mark field as initialized
                    state.assigned.insert(field.clone());
                    // Track let field assignments
                    if ctx.let_fields.contains(field) {
                        state.let_assigned.insert(field.clone());
                    }
                }
            }

            // Analyze target as assignment target (for nested field access etc)
            state = analyze_expression(target, state, true, ctx);
        }
        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Analyze condition first
            state = analyze_expression(condition, state, false, ctx);

            // Save state before branches
            let pre_branch_state = state.clone();

            // Analyze then branch
            ctx.state = pre_branch_state.clone();
            let mut then_state = pre_branch_state.clone();
            for stmt in then_branch {
                if then_state.diverged {
                    break;
                }
                then_state = analyze_statement(stmt, then_state, ctx);
            }
            if !then_state.diverged {
                if let Some(value) = then_value {
                    then_state = analyze_expression(value, then_state, false, ctx);
                }
            }

            // Analyze else branch if present
            let else_state = if let Some(else_branch) = else_branch {
                ctx.state = pre_branch_state.clone();
                let mut else_state = pre_branch_state.clone();
                match else_branch {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if else_state.diverged {
                                break;
                            }
                            else_state = analyze_statement(stmt, else_state, ctx);
                        }
                        if !else_state.diverged {
                            if let Some(value) = value {
                                else_state = analyze_expression(value, else_state, false, ctx);
                            }
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(if_expr) => {
                        else_state = analyze_expression(if_expr, else_state, false, ctx);
                    }
                }
                else_state
            } else {
                // No else branch - the "else" path has the pre-branch state
                // (if condition is false, no initializations happen)
                pre_branch_state
            };

            // Merge the two branch states
            state = then_state.merge(else_state);
        }
        ExprKind::While {
            condition, body, ..
        } => {
            // Analyze condition
            state = analyze_expression(condition, state, false, ctx);

            // While loop body may execute zero times, so we analyze it but
            // don't rely on its initializations for the continuation.
            // We still need to check for errors inside the body.
            let pre_loop_state = state.clone();
            ctx.state = state.clone();
            let mut body_state = state.clone();
            for stmt in body {
                if body_state.diverged {
                    break;
                }
                body_state = analyze_statement(stmt, body_state, ctx);
            }

            // The state after while is the pre-loop state (conservative)
            // because the loop might not execute at all
            state = pre_loop_state;
        }
        ExprKind::Loop { body, .. } => {
            // Loop always executes at least once
            // We need to track the state at each break point

            // Analyze the body - collect states at break points
            ctx.state = state.clone();
            let mut body_state = state.clone();
            let mut break_states: Vec<InitState> = Vec::new();

            for stmt in body {
                if body_state.diverged {
                    // If we hit a break, save the state before divergence
                    // The diverged flag was set by the break itself
                    break;
                }

                // Save state before analyzing the statement
                let pre_stmt_state = body_state.clone();
                body_state = analyze_statement(stmt, body_state, ctx);

                // If this statement caused divergence via break, record the state
                // We use pre_stmt_state because that's what was initialized before break
                if body_state.diverged && contains_break_at_top_level(&stmt.kind) {
                    // For break, we want the state AFTER the statements before the break
                    // but the break itself doesn't initialize anything
                    // Actually we need the state from within the statement up to the break
                    // For simplicity, just record body_state (with diverged cleared for the merge)
                    let mut break_state = body_state.clone();
                    break_state.diverged = false;
                    break_states.push(break_state);
                }
            }

            // Determine the exit state
            if break_states.is_empty() && !body_state.diverged {
                // No breaks and no return - infinite loop, code after unreachable
                state.diverged = true;
            } else if break_states.is_empty() {
                // No breaks but body diverged (all paths return) - propagate divergence
                state = body_state;
            } else {
                // Has breaks - merge all break states
                // Since loop executes at least once, we use the break states
                let mut merged = break_states.pop().unwrap();
                for bs in break_states {
                    merged = merged.merge(bs);
                }
                state = merged;
            }
        }
        ExprKind::Break { .. } => {
            // Break exits the current loop - mark as diverged for this path
            state.diverged = true;
        }
        ExprKind::Continue { .. } => {
            // Continue goes to next iteration - mark as diverged for this path
            state.diverged = true;
        }
        ExprKind::Return { value } => {
            // Analyze return value if present
            if let Some(val) = value {
                state = analyze_expression(val, state, false, ctx);
            }

            // Check that all fields are initialized before return
            let uninitialized: Vec<String> = ctx
                .all_fields
                .iter()
                .filter(|f| !state.assigned.contains(*f))
                .cloned()
                .collect();

            if !uninitialized.is_empty() {
                ctx.errors
                    .push(InitializerError::ReturnBeforeFullyInitialized {
                        span: expr.span.clone(),
                        uninitialized,
                    });
            }

            // Mark as diverged
            state.diverged = true;
        }
        ExprKind::Error => {}
    }

    state
}

/// Check if a statement kind contains a break at the top level (not nested in another loop)
fn contains_break_at_top_level(kind: &StatementKind) -> bool {
    match kind {
        StatementKind::Expr(expr) => expr_contains_break_at_top_level(&expr.kind),
        StatementKind::Binding { value: Some(expr), .. } => {
            expr_contains_break_at_top_level(&expr.kind)
        }
        StatementKind::Binding { value: None, .. } => false,
    }
}

/// Check if an expression contains a break at the top level
fn expr_contains_break_at_top_level(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Break { .. } => true,
        ExprKind::If { then_branch, then_value, else_branch, .. } => {
            // Check then branch
            for stmt in then_branch {
                if contains_break_at_top_level(&stmt.kind) {
                    return true;
                }
            }
            if let Some(val) = then_value {
                if expr_contains_break_at_top_level(&val.kind) {
                    return true;
                }
            }
            // Check else branch
            if let Some(else_b) = else_branch {
                match else_b {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if contains_break_at_top_level(&stmt.kind) {
                                return true;
                            }
                        }
                        if let Some(val) = value {
                            if expr_contains_break_at_top_level(&val.kind) {
                                return true;
                            }
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(if_expr) => {
                        if expr_contains_break_at_top_level(&if_expr.kind) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        // Don't recurse into nested loops - breaks there don't affect outer loop
        ExprKind::While { .. } | ExprKind::Loop { .. } => false,
        _ => false,
    }
}

/// Check if an expression is a reference to `self`
///
/// In initializers and instance methods, `self` is always local 0.
fn is_self_expr(expr: &Expression) -> bool {
    use crate::validation::assignment_validation::is_self_expr as check_self;
    check_self(expr)
}

/// Check if a field is mutable (var vs let)
fn is_field_mutable(field: &Arc<dyn Symbol<KestrelLanguage>>) -> bool {
    use kestrel_semantic_tree::symbol::field::FieldSymbol;
    if let Some(field_sym) = field.as_ref().downcast_ref::<FieldSymbol>() {
        field_sym.is_mutable()
    } else {
        // If we can't downcast, assume mutable to avoid false positives
        true
    }
}

/// Get the executable body from a symbol
fn get_executable_body(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<CodeBlock> {
    let behaviors = symbol.metadata().behaviors();
    for b in behaviors.iter() {
        if matches!(b.kind(), KestrelBehaviorKind::Executable) {
            if let Some(exec) = b.as_ref().downcast_ref::<ExecutableBehavior>() {
                return Some(exec.body().clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_state_merge_both_initialized() {
        let mut state1 = InitState::new();
        state1.assigned.insert("x".to_string());
        state1.assigned.insert("y".to_string());

        let mut state2 = InitState::new();
        state2.assigned.insert("x".to_string());
        state2.assigned.insert("z".to_string());

        let merged = state1.merge(state2);

        // Only x is in both branches
        assert!(merged.assigned.contains("x"));
        assert!(!merged.assigned.contains("y"));
        assert!(!merged.assigned.contains("z"));
        assert!(!merged.diverged);
    }

    #[test]
    fn test_init_state_merge_one_diverged() {
        let mut state1 = InitState::new();
        state1.assigned.insert("x".to_string());
        state1.diverged = true; // This branch returns

        let mut state2 = InitState::new();
        state2.assigned.insert("y".to_string());

        let merged = state1.merge(state2);

        // state1 diverged, so we use state2
        assert!(!merged.assigned.contains("x"));
        assert!(merged.assigned.contains("y"));
        assert!(!merged.diverged);
    }

    #[test]
    fn test_init_state_merge_both_diverged() {
        let mut state1 = InitState::new();
        state1.assigned.insert("x".to_string());
        state1.diverged = true;

        let mut state2 = InitState::new();
        state2.assigned.insert("x".to_string());
        state2.assigned.insert("y".to_string());
        state2.diverged = true;

        let merged = state1.merge(state2);

        // Both diverged - intersection of assigned, still diverged
        assert!(merged.assigned.contains("x"));
        assert!(!merged.assigned.contains("y"));
        assert!(merged.diverged);
    }

    #[test]
    fn test_let_field_tracking_across_branches() {
        let mut state = InitState::new();

        // Simulate: if cond { self.x = 1 } else { self.x = 2 }
        let mut then_state = state.clone();
        then_state.assigned.insert("x".to_string());
        then_state.let_assigned.insert("x".to_string());

        let mut else_state = state.clone();
        else_state.assigned.insert("x".to_string());
        else_state.let_assigned.insert("x".to_string());

        let merged = then_state.merge(else_state);

        // x is assigned in both branches
        assert!(merged.assigned.contains("x"));
        // let_assigned is union - x was assigned in both
        assert!(merged.let_assigned.contains("x"));
    }
}
