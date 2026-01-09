//! Analyzer that verifies initializer bodies correctly initialize all fields,
//! restricts double-assignment to `let` fields, forbids reading fields before
//! initialization, and using self/return before full initialization.

use std::collections::HashSet;
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use diagnostics::{InitializerError, UninitializedFieldsError};

use kestrel_semantic_model::{ExecutableBodyFor, StructFields};
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

pub struct InitializerVerificationAnalyzer;

impl InitializerVerificationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for InitializerVerificationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for InitializerVerificationAnalyzer {
    fn name(&self) -> &'static str {
        "initializer_verification"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Initializer {
            return;
        }
        validate_initializer(symbol, ctx);
    }
}

fn validate_initializer(symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
    // Parent must be a struct to know fields
    let Some(parent) = symbol.metadata().parent() else {
        return;
    };
    if parent.metadata().kind() != KestrelSymbolKind::Struct {
        return;
    }

    // Collect field names and mutability
    let struct_id = parent.metadata().id();
    let fields: Vec<FieldInfo> = ctx
        .model
        .query(StructFields { struct_id })
        .into_iter()
        .map(|field| FieldInfo {
            name: field.name,
            is_let: !field.is_mutable,
        })
        .collect();

    // Get initializer body
    let symbol_id = symbol.metadata().id();
    let Some(body) = ctx.model.query(ExecutableBodyFor { symbol_id }) else {
        return;
    };

    let all_fields: HashSet<String> = fields.iter().map(|f| f.name.clone()).collect();
    let let_fields: HashSet<String> = fields
        .iter()
        .filter(|f| f.is_let)
        .map(|f| f.name.clone())
        .collect();

    let mut vctx = VerificationContext {
        all_fields: all_fields.clone(),
        let_fields,
        state: InitState::new(),
        errors: Vec::new(),
    };

    let final_state = analyze_block(&body.statements, body.yield_expr.as_deref(), &mut vctx);

    if !final_state.diverged {
        let uninitialized: Vec<&String> = all_fields
            .iter()
            .filter(|f| !final_state.assigned.contains(*f))
            .collect();
        if !uninitialized.is_empty() {
            let span = symbol.metadata().span().clone();
            let field_list = uninitialized
                .iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ");
            ctx.report(UninitializedFieldsError {
                span,
                fields: field_list,
            });
        }
    }

    for error in vctx.errors {
        ctx.report(error);
    }
}

struct FieldInfo {
    name: String,
    is_let: bool,
}

#[derive(Clone, Debug)]
struct InitState {
    assigned: HashSet<String>,
    let_assigned: HashSet<String>,
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
}

impl InitState {
    fn merge(self, other: InitState) -> InitState {
        if self.diverged && other.diverged {
            InitState {
                assigned: self
                    .assigned
                    .intersection(&other.assigned)
                    .cloned()
                    .collect(),
                let_assigned: self
                    .let_assigned
                    .union(&other.let_assigned)
                    .cloned()
                    .collect(),
                diverged: true,
            }
        } else if self.diverged {
            other
        } else if other.diverged {
            self
        } else {
            InitState {
                assigned: self
                    .assigned
                    .intersection(&other.assigned)
                    .cloned()
                    .collect(),
                let_assigned: self
                    .let_assigned
                    .union(&other.let_assigned)
                    .cloned()
                    .collect(),
                diverged: false,
            }
        }
    }
}

struct VerificationContext {
    all_fields: HashSet<String>,
    let_fields: HashSet<String>,
    state: InitState,
    errors: Vec<InitializerError>,
}

fn analyze_block(
    statements: &[Statement],
    yield_expr: Option<&Expression>,
    ctx: &mut VerificationContext,
) -> InitState {
    let mut state = ctx.state.clone();
    for stmt in statements {
        if state.diverged {
            break;
        }
        state = analyze_statement(stmt, state, ctx);
    }
    if !state.diverged {
        if let Some(expr) = yield_expr {
            state = analyze_expression(expr, state, false, ctx);
        }
    }
    state
}

fn analyze_statement(
    stmt: &Statement,
    mut state: InitState,
    ctx: &mut VerificationContext,
) -> InitState {
    match &stmt.kind {
        StatementKind::Binding { pattern: _, value } => {
            if let Some(expr) = value {
                state = analyze_expression(expr, state, false, ctx);
            }
        }
        StatementKind::Expr(expr) => {
            state = analyze_expression(expr, state, false, ctx);
        }
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Analyze each condition
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        state = analyze_expression(expr, state, false, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        state = analyze_expression(value, state, false, ctx);
                    }
                }
            }
            // Analyze the else block (it diverges so we don't merge state)
            let else_state = analyze_block(
                &else_block.statements,
                else_block.yield_expr.as_deref(),
                ctx,
            );
            let _ = else_state;
        }
        StatementKind::Deinit { .. } => {
            // Deinit doesn't involve field initialization
        }
    }
    state
}

fn analyze_expression(
    expr: &Expression,
    mut state: InitState,
    is_assignment_target: bool,
    ctx: &mut VerificationContext,
) -> InitState {
    match &expr.kind {
        ExprKind::FieldAccess { object, field } => {
            if is_self_expr(object) {
                if !is_assignment_target {
                    if !state.assigned.contains(field) {
                        ctx.errors.push(InitializerError::FieldReadBeforeAssigned {
                            span: expr.span.clone(),
                            field_name: field.clone(),
                        });
                    }
                }
            } else {
                state = analyze_expression(object, state, false, ctx);
            }
        }
        ExprKind::Call {
            callee, arguments, ..
        } => {
            if let ExprKind::MethodRef { receiver, .. } = &callee.kind {
                if is_self_expr(receiver) {
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
            state = analyze_expression(callee, state, false, ctx);
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
        }
        ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::OverloadedRef(_) => {}
        ExprKind::MethodRef { receiver, .. } => {
            state = analyze_expression(receiver, state, false, ctx);
        }
        ExprKind::PrimitiveMethodRef { receiver, .. } => {
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
        ExprKind::DeferredMethodCall {
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
        ExprKind::DelegatingInit { arguments, .. } => {
            // Analyze the arguments first
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
            // After a delegating init call, ALL fields are considered initialized
            // because the delegated initializer handles field initialization
            for field in &ctx.all_fields {
                state.assigned.insert(field.clone());
            }
            for field in &ctx.let_fields {
                state.let_assigned.insert(field.clone());
            }
        }
        ExprKind::Assignment { target, value } => {
            state = analyze_expression(value, state, false, ctx);
            if let ExprKind::FieldAccess { object, field } = &target.kind {
                if is_self_expr(object) {
                    if ctx.let_fields.contains(field) && state.let_assigned.contains(field) {
                        ctx.errors.push(InitializerError::LetFieldAssignedTwice {
                            span: target.span.clone(),
                            field_name: field.clone(),
                        });
                    }
                    state.assigned.insert(field.clone());
                    if ctx.let_fields.contains(field) {
                        state.let_assigned.insert(field.clone());
                    }
                }
            }
            state = analyze_expression(target, state, true, ctx);
        }
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Analyze all conditions
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(e) => {
                        state = analyze_expression(e, state, false, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        state = analyze_expression(value, state, false, ctx);
                    }
                }
            }
            let pre = state.clone();
            // then
            ctx.state = pre.clone();
            let mut then_state = pre.clone();
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
            // else
            let else_state = if let Some(else_branch) = else_branch {
                ctx.state = pre.clone();
                let mut else_state = pre.clone();
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
                pre.clone()
            };
            state = then_state.merge(else_state);
        }
        ExprKind::While {
            condition, body, ..
        } => {
            state = analyze_expression(condition, state, false, ctx);
            // Body may execute zero times; so it doesn't contribute guaranteed initialization
            let mut body_state = state.clone();
            for stmt in body {
                if body_state.diverged {
                    break;
                }
                body_state = analyze_statement(stmt, body_state, ctx);
            }
            // Ignore yield for while
        }
        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(e) => {
                        state = analyze_expression(e, state, false, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        state = analyze_expression(value, state, false, ctx);
                    }
                }
            }
            // Body may execute zero times; so it doesn't contribute guaranteed initialization
            let mut body_state = state.clone();
            for stmt in body {
                if body_state.diverged {
                    break;
                }
                body_state = analyze_statement(stmt, body_state, ctx);
            }
            // Ignore yield for while-let
        }
        ExprKind::Loop { body, .. } => {
            let mut break_states: Vec<InitState> = Vec::new();
            let mut body_state = state.clone();
            for stmt in body {
                if body_state.diverged {
                    break;
                }

                body_state = analyze_statement(stmt, body_state, ctx);

                if body_state.diverged && contains_break_at_top_level(&stmt.kind) {
                    let mut break_state = body_state.clone();
                    break_state.diverged = false;
                    break_states.push(break_state);
                }
            }
            if break_states.is_empty() && !body_state.diverged {
                state.diverged = true;
            } else if break_states.is_empty() {
                state = body_state;
            } else {
                let mut merged = break_states.pop().unwrap();
                for bs in break_states {
                    merged = merged.merge(bs);
                }
                state = merged;
            }
        }
        ExprKind::Break { .. } | ExprKind::Continue { .. } => {
            state.diverged = true;
        }
        ExprKind::Return { value } => {
            if let Some(val) = value {
                state = analyze_expression(val, state, false, ctx);
            }
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
            state.diverged = true;
        }
        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // Analyze closure body - closures capture variables but don't affect init state
            for stmt in body {
                let _ = analyze_statement(stmt, state.clone(), ctx);
            }
            if let Some(tail) = tail_expr {
                let _ = analyze_expression(tail, state.clone(), false, ctx);
            }
            // Closures don't change the initialization state of the enclosing scope
        }
        ExprKind::EnumCase { .. } => {}
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    state = analyze_expression(&arg.value, state, false, ctx);
                }
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            state = analyze_expression(scrutinee, state, false, ctx);
            for arm in arms {
                let mut arm_state = state.clone();
                if let Some(guard) = &arm.guard {
                    arm_state = analyze_expression(guard, arm_state, false, ctx);
                }
                arm_state = analyze_expression(&arm.body, arm_state, false, ctx);
            }
        }
        ExprKind::Block { statements, value } => {
            for stmt in statements {
                if state.diverged {
                    break;
                }
                state = analyze_statement(stmt, state, ctx);
            }
            if !state.diverged {
                if let Some(val) = value {
                    state = analyze_expression(val, state, false, ctx);
                }
            }
        }
        ExprKind::LangIntrinsic {
            arguments,
            intrinsic,
        } => {
            use kestrel_semantic_tree::expr::LangIntrinsic;
            for arg in arguments {
                state = analyze_expression(&arg.value, state, false, ctx);
            }
            // Check if this intrinsic diverges
            match intrinsic {
                LangIntrinsic::PanicUnwind => state.diverged = true,
                LangIntrinsic::Cast { .. } => {} // Cast returns normally
            }
        }
        ExprKind::LangIntrinsicRef(_) | ExprKind::Error => {}
    }
    state
}

fn contains_break_at_top_level(kind: &StatementKind) -> bool {
    match kind {
        StatementKind::Expr(expr) => expr_contains_break_at_top_level(&expr.kind),
        StatementKind::Binding {
            value: Some(expr), ..
        } => expr_contains_break_at_top_level(&expr.kind),
        StatementKind::Binding { value: None, .. } => false,
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        if expr_contains_break_at_top_level(&expr.kind) {
                            return true;
                        }
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        if expr_contains_break_at_top_level(&value.kind) {
                            return true;
                        }
                    }
                }
            }
            for stmt in &else_block.statements {
                if contains_break_at_top_level(&stmt.kind) {
                    return true;
                }
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                if expr_contains_break_at_top_level(&yield_expr.kind) {
                    return true;
                }
            }
            false
        }
        StatementKind::Deinit { .. } => {
            // Deinit doesn't contain break
            false
        }
    }
}

fn expr_contains_break_at_top_level(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Break { .. } => true,
        ExprKind::If {
            then_branch,
            then_value,
            else_branch,
            ..
        } => {
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
        ExprKind::While { .. } | ExprKind::WhileLet { .. } | ExprKind::Loop { .. } => false,
        _ => false,
    }
}

fn is_self_expr(expr: &Expression) -> bool {
    match &expr.kind {
        ExprKind::LocalRef(local_id) => local_id.index() == 0,
        _ => false,
    }
}

pub mod diagnostics;
