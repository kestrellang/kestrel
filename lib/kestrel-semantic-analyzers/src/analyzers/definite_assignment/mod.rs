use std::collections::HashSet;
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;
use kestrel_semantic_model::ExecutableBodyFor;
use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::{ExprKind, Expression, ElseBranch};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::local::LocalId;
use semantic_tree::symbol::Symbol;
use diagnostics::UninitializedVariableAccessError;
use kestrel_semantic_model::LocalName;

pub struct DefiniteAssignmentAnalyzer;

impl DefiniteAssignmentAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefiniteAssignmentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DefiniteAssignmentAnalyzer {
    fn name(&self) -> &'static str {
        "definite_assignment"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        if kind != KestrelSymbolKind::Function && kind != KestrelSymbolKind::Initializer {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let Some(body) = ctx.model.query(ExecutableBodyFor { symbol_id }) else {
            return;
        };

        let mut assigned = HashSet::new();

        // Parameters and 'self' (for instance methods/initializers) are always initialized.
        use kestrel_semantic_tree::behavior::callable::CallableBehavior;
        if let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() {
            let mut count = callable.arity();
            if callable.is_instance_method() {
                count += 1;
            }
            for i in 0..count {
                assigned.insert(LocalId::new(i));
            }
        }

        let mut vctx = VerificationContext {
            assigned,
            errors: Vec::new(),
            container_id: symbol_id,
            model: ctx.model,
        };

        analyze_block(&body, &mut vctx);

        for error in vctx.errors {
            ctx.report(error);
        }
    }
}

struct VerificationContext<'a> {
    assigned: HashSet<LocalId>,
    errors: Vec<UninitializedVariableAccessError>,
    container_id: semantic_tree::symbol::SymbolId,
    model: &'a kestrel_semantic_model::SemanticModel,
}

#[derive(Clone, Debug)]
struct State {
    assigned: HashSet<LocalId>,
    diverged: bool,
}

fn analyze_block(body: &CodeBlock, ctx: &mut VerificationContext) -> State {
    let mut state = State {
        assigned: ctx.assigned.clone(),
        diverged: false,
    };

    for stmt in &body.statements {
        if state.diverged {
            break;
        }
        state = analyze_statement(stmt, state, ctx);
    }

    if !state.diverged {
        if let Some(yield_expr) = body.yield_expr() {
            state = analyze_expression(yield_expr, state, false, ctx);
        }
    }

    state
}

fn analyze_statement(stmt: &Statement, mut state: State, ctx: &mut VerificationContext) -> State {
    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            if let Some(value_expr) = value {
                state = analyze_expression(value_expr, state, false, ctx);
                // Mark variables in pattern as initialized
                use kestrel_semantic_tree::pattern::PatternKind;
                match &pattern.kind {
                    PatternKind::Local { local_id, .. } => {
                        state.assigned.insert(*local_id);
                    }
                    PatternKind::Error => {}
                }
            }
        }
        StatementKind::Expr(expr) => {
            state = analyze_expression(expr, state, false, ctx);
        }
    }
    state
}

fn analyze_expression(
    expr: &Expression,
    mut state: State,
    is_assignment_target: bool,
    ctx: &mut VerificationContext,
) -> State {
    match &expr.kind {
        ExprKind::LocalRef(local_id) => {
            if !is_assignment_target && !state.assigned.contains(local_id) {
                let name = ctx.model.query(LocalName {
                    container_id: ctx.container_id,
                    local_id: *local_id,
                }).unwrap_or_else(|| "<unknown>".to_string());

                ctx.errors.push(UninitializedVariableAccessError {
                    span: expr.span.clone(),
                    variable_name: name,
                });
            }
        }
        ExprKind::Assignment { target, value } => {
            state = analyze_expression(value, state, false, ctx);
            // If target is a LocalRef, mark it as assigned
            if let ExprKind::LocalRef(local_id) = &target.kind {
                state.assigned.insert(*local_id);
            }
            state = analyze_expression(target, state, true, ctx);
        }
        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            state = analyze_expression(condition, state, false, ctx);
            let pre_if_assigned = state.assigned.clone();

            // Analyze then branch
            let mut then_state = State {
                assigned: pre_if_assigned.clone(),
                diverged: false,
            };
            for stmt in then_branch {
                if then_state.diverged { break; }
                then_state = analyze_statement(stmt, then_state, ctx);
            }
            if !then_state.diverged {
                if let Some(v) = then_value {
                    then_state = analyze_expression(v, then_state, false, ctx);
                }
            }

            // Analyze else branch
            let else_state = if let Some(else_b) = else_branch {
                let mut es = State {
                    assigned: pre_if_assigned.clone(),
                    diverged: false,
                };
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if es.diverged { break; }
                            es = analyze_statement(stmt, es, ctx);
                        }
                        if !es.diverged {
                            if let Some(v) = value {
                                es = analyze_expression(v, es, false, ctx);
                            }
                        }
                    }
                    ElseBranch::ElseIf(if_expr) => {
                        es = analyze_expression(if_expr, es, false, ctx);
                    }
                }
                es
            } else {
                State {
                    assigned: pre_if_assigned,
                    diverged: false,
                }
            };

            // Merge states
            if then_state.diverged && else_state.diverged {
                state.diverged = true;
                state.assigned = then_state.assigned.intersection(&else_state.assigned).cloned().collect();
            } else if then_state.diverged {
                state = else_state;
            } else if else_state.diverged {
                state = then_state;
            } else {
                state.assigned = then_state.assigned.intersection(&else_state.assigned).cloned().collect();
            }
        }
        ExprKind::While { condition, body, .. } => {
            state = analyze_expression(condition, state, false, ctx);
            let mut body_state = State {
                assigned: state.assigned.clone(),
                diverged: false,
            };
            for stmt in body {
                if body_state.diverged { break; }
                body_state = analyze_statement(stmt, body_state, ctx);
            }
        }
        ExprKind::Loop { body, .. } => {
            let mut body_state = State {
                assigned: state.assigned.clone(),
                diverged: false,
            };
            for stmt in body {
                if body_state.diverged { break; }
                body_state = analyze_statement(stmt, body_state, ctx);
            }
            // For simplicity, we don't assume anything about loop exit state yet
        }
        ExprKind::Return { value } => {
            if let Some(v) = value {
                state = analyze_expression(v, state, false, ctx);
            }
            state.diverged = true;
        }
        ExprKind::Break { .. } | ExprKind::Continue { .. } => {
            state.diverged = true;
        }
        ExprKind::Literal(_) | ExprKind::SymbolRef(_) | ExprKind::TypeRef(_) | ExprKind::TypeParameterRef(_) | ExprKind::Error | ExprKind::OverloadedRef(_) => {}
        ExprKind::Array(exprs) | ExprKind::Tuple(exprs) => {
            for e in exprs {
                state = analyze_expression(e, state, false, ctx);
            }
        }
        ExprKind::Grouping(inner) => {
            state = analyze_expression(inner, state, false, ctx);
        }
        ExprKind::FieldAccess { object, .. } => {
            state = analyze_expression(object, state, false, ctx);
        }
        ExprKind::TupleIndex { tuple, .. } => {
            state = analyze_expression(tuple, state, false, ctx);
        }
        ExprKind::MethodRef { receiver, .. } => {
            state = analyze_expression(receiver, state, false, ctx);
        }
        ExprKind::Call { callee, arguments, .. } => {
            state = analyze_expression(callee, state, false, ctx);
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
        }
        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
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
    }
    state
}

pub mod diagnostics;
