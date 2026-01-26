use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::ExecutableBodyFor;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;

mod diagnostics;
use diagnostics::MissingReturnError;

pub struct ExhaustiveReturnAnalyzer;

impl ExhaustiveReturnAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for ExhaustiveReturnAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ExhaustiveReturnAnalyzer {
    fn name(&self) -> &'static str {
        "exhaustive_return"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only check functions
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        // Downcast to FunctionSymbol
        let Ok(func) = symbol.clone().downcast_arc::<FunctionSymbol>() else {
            return;
        };

        // Skip unit return types
        let return_ty = func.return_type();
        if is_unit_type(return_ty.kind()) {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let Some(body) = ctx.model.query(ExecutableBodyFor { symbol_id }) else {
            return;
        };
        let state = analyze_block(&body.statements, body.yield_expr.as_deref());
        if state.definitely_returns() {
            return;
        }

        let func_name = symbol.metadata().name().value.clone();
        let span = symbol.metadata().declaration_span().clone();
        ctx.report(MissingReturnError { span, func_name });
    }
}

fn is_unit_type(kind: &TyKind) -> bool {
    match kind {
        TyKind::Unit => true,
        TyKind::Tuple(elements) => elements.is_empty(),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnState {
    Returns,
    Diverges,
    MayFallThrough,
}

impl ReturnState {
    fn definitely_returns(self) -> bool {
        matches!(self, ReturnState::Returns | ReturnState::Diverges)
    }
    fn merge(self, other: ReturnState) -> ReturnState {
        match (self, other) {
            (ReturnState::Returns, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Returns, ReturnState::Diverges) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Diverges) => ReturnState::Diverges,
            _ => ReturnState::MayFallThrough,
        }
    }
}

fn analyze_block(statements: &[Statement], yield_expr: Option<&Expression>) -> ReturnState {
    let mut state = ReturnState::MayFallThrough;
    for stmt in statements {
        if state.definitely_returns() {
            return state;
        }
        state = analyze_statement(stmt);
    }
    if !state.definitely_returns()
        && let Some(expr) = yield_expr
    {
        let expr_state = analyze_expression(expr);
        if expr_state.definitely_returns() {
            return expr_state;
        }
        return ReturnState::Returns;
    }
    state
}

fn analyze_statement(stmt: &Statement) -> ReturnState {
    match &stmt.kind {
        StatementKind::Binding {
            value: Some(expr), ..
        } => analyze_expression(expr),
        StatementKind::Binding { value: None, .. } => ReturnState::MayFallThrough,
        StatementKind::Expr(expr) => analyze_expression(expr),
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Check if any condition expression diverges
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        let state = analyze_expression(expr);
                        if state.definitely_returns() {
                            return state;
                        }
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        let state = analyze_expression(value);
                        if state.definitely_returns() {
                            return state;
                        }
                    },
                }
            }
            // The else block must diverge - if it does, control continues after guard-let
            let else_state =
                analyze_block(&else_block.statements, else_block.yield_expr.as_deref());
            if else_state.definitely_returns() {
                // The else block diverges, so control continues after guard-let
                ReturnState::MayFallThrough
            } else {
                // If the else block doesn't diverge, that's an error (reported elsewhere)
                // but for return analysis, we still may fall through
                ReturnState::MayFallThrough
            }
        },
        StatementKind::Deinit { .. } => {
            // Deinit is a simple statement that doesn't return or diverge
            ReturnState::MayFallThrough
        },
    }
}

fn analyze_expression(expr: &Expression) -> ReturnState {
    match &expr.kind {
        ExprKind::Return { .. } => ReturnState::Returns,
        ExprKind::Break { .. } | ExprKind::Continue { .. } => ReturnState::Diverges,
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            for cond in conditions {
                let cond_state = match cond {
                    IfCondition::Expr(e) => analyze_expression(e),
                    IfCondition::Let { value, .. } => analyze_expression(value),
                };
                if cond_state.definitely_returns() {
                    return cond_state;
                }
            }
            let then_state = analyze_block(then_branch, then_value.as_deref());
            let else_state = if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        analyze_block(statements, value.as_deref())
                    },
                    ElseBranch::ElseIf(if_expr) => analyze_expression(if_expr),
                }
            } else {
                ReturnState::MayFallThrough
            };
            then_state.merge(else_state)
        },
        ExprKind::While {
            condition, body, ..
        } => {
            let cond_state = analyze_expression(condition);
            if cond_state.definitely_returns() {
                return cond_state;
            }
            let _ = analyze_block(body, None);
            ReturnState::MayFallThrough
        },
        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        let state = analyze_expression(expr);
                        if state.definitely_returns() {
                            return state;
                        }
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        let state = analyze_expression(value);
                        if state.definitely_returns() {
                            return state;
                        }
                    },
                }
            }
            let _ = analyze_block(body, None);
            ReturnState::MayFallThrough
        },
        ExprKind::Loop { body, .. } => {
            let mut body_state = ReturnState::MayFallThrough;
            let mut has_break = false;
            for stmt in body {
                if body_state.definitely_returns() {
                    break;
                }
                body_state = analyze_statement(stmt);
                if statement_contains_break(&stmt.kind) {
                    has_break = true;
                }
            }
            if body_state == ReturnState::Returns {
                return ReturnState::Returns;
            }
            if !has_break && body_state != ReturnState::Returns {
                return ReturnState::Diverges;
            }
            ReturnState::MayFallThrough
        },
        ExprKind::Call {
            callee, arguments, ..
        } => {
            let state = analyze_expression(callee);
            if state.definitely_returns() {
                return state;
            }
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::Assignment { target, value } => {
            let state = analyze_expression(value);
            if state.definitely_returns() {
                return state;
            }
            analyze_expression(target)
        },
        ExprKind::Grouping(inner) => analyze_expression(inner),
        ExprKind::Array(elements) => {
            for e in elements {
                let s = analyze_expression(e);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::Tuple(elements) => {
            for e in elements {
                let s = analyze_expression(e);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::Dictionary(pairs) => {
            for (k, v) in pairs {
                let s = analyze_expression(k);
                if s.definitely_returns() {
                    return s;
                }
                let s = analyze_expression(v);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::FieldAccess { object, .. } => analyze_expression(object),
        ExprKind::TupleIndex { tuple, .. } => analyze_expression(tuple),
        ExprKind::MethodRef { receiver, .. } => analyze_expression(receiver),
        ExprKind::PrimitiveMethodRef { receiver, .. } => analyze_expression(receiver),
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            let s = analyze_expression(receiver);
            if s.definitely_returns() {
                return s;
            }
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            let s = analyze_expression(receiver);
            if s.definitely_returns() {
                return s;
            }
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::DeferredStaticCall { arguments, .. } => {
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                let s = analyze_expression(&arg.value);
                if s.definitely_returns() {
                    return s;
                }
            }
            ReturnState::MayFallThrough
        },
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    let s = analyze_expression(&arg.value);
                    if s.definitely_returns() {
                        return s;
                    }
                }
            }
            ReturnState::MayFallThrough
        },
        // Lang intrinsic - check which intrinsic
        ExprKind::LangIntrinsic { intrinsic, .. } => {
            use kestrel_semantic_tree::expr::LangIntrinsic;
            match intrinsic {
                // panic_unwind never returns
                LangIntrinsic::PanicUnwind => ReturnState::Returns,
                // All other intrinsics return a value normally
                _ => ReturnState::MayFallThrough,
            }
        },

        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Closure { .. }
        | ExprKind::LangIntrinsicRef(_)
        | ExprKind::SubscriptCall { .. }
        | ExprKind::Error => ReturnState::MayFallThrough,

        // Match expression - all arms must return for the match to be exhaustive
        ExprKind::Match { arms, .. } => {
            if arms.is_empty() {
                ReturnState::MayFallThrough
            } else if arms.iter().all(|arm| {
                let body_state = analyze_expression(&arm.body);
                body_state.definitely_returns()
            }) {
                ReturnState::Returns
            } else {
                ReturnState::MayFallThrough
            }
        },

        // Block expression - analyze statements and value
        ExprKind::Block { statements, value } => analyze_block(statements, value.as_deref()),
    }
}

fn statement_contains_break(kind: &StatementKind) -> bool {
    match kind {
        StatementKind::Expr(expr) => expr_contains_break(&expr.kind),
        StatementKind::Binding {
            value: Some(expr), ..
        } => expr_contains_break(&expr.kind),
        StatementKind::Binding { value: None, .. } => false,
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        if expr_contains_break(&expr.kind) {
                            return true;
                        }
                    },
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        if expr_contains_break(&value.kind) {
                            return true;
                        }
                    },
                }
            }
            for stmt in &else_block.statements {
                if statement_contains_break(&stmt.kind) {
                    return true;
                }
            }
            if let Some(yield_expr) = &else_block.yield_expr
                && expr_contains_break(&yield_expr.kind)
            {
                return true;
            }
            false
        },
        StatementKind::Deinit { .. } => {
            // Deinit doesn't contain break
            false
        },
    }
}

fn expr_contains_break(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Break { .. } => true,
        ExprKind::If {
            then_branch,
            then_value,
            else_branch,
            ..
        } => {
            for stmt in then_branch {
                if statement_contains_break(&stmt.kind) {
                    return true;
                }
            }
            if let Some(val) = then_value
                && expr_contains_break(&val.kind)
            {
                return true;
            }
            if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if statement_contains_break(&stmt.kind) {
                                return true;
                            }
                        }
                        if let Some(val) = value
                            && expr_contains_break(&val.kind)
                        {
                            return true;
                        }
                    },
                    ElseBranch::ElseIf(if_expr) => {
                        if expr_contains_break(&if_expr.kind) {
                            return true;
                        }
                    },
                }
            }
            false
        },
        // Don't recurse into nested loops
        ExprKind::While { .. } | ExprKind::WhileLet { .. } | ExprKind::Loop { .. } => false,
        _ => false,
    }
}
