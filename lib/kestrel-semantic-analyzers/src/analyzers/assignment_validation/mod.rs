use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use diagnostics::{
    CannotAssignToExpressionError, CannotAssignToImmutableError, CannotAssignToImmutableFieldError,
};

use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::local::LocalId;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

pub struct AssignmentValidationAnalyzer;

impl AssignmentValidationAnalyzer { pub fn new() -> Self { Self } }
impl Default for AssignmentValidationAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for AssignmentValidationAnalyzer {
    fn name(&self) -> &'static str { "assignment_validation" }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        let ExprKind::Assignment { target, .. } = &expr.kind else { return; };

        // Determine current container (function or initializer)
        let mut func: Option<&FunctionSymbol> = None;
        let mut init: Option<&InitializerSymbol> = None;
        let current_sym = ctx.current_symbol();
        if let Some(sym) = &current_sym {
            if let Some(f) = sym.as_ref().downcast_ref::<FunctionSymbol>() { func = Some(f); }
            if let Some(i) = sym.as_ref().downcast_ref::<InitializerSymbol>() { init = Some(i); }
        }

        let errors = validate_assignment_target(target, func, init);
        for e in errors {
            match e {
                AssignmentError::ImmutableVar(err) => ctx.report(err),
                AssignmentError::ImmutableField(err) => ctx.report(err),
                AssignmentError::InvalidTarget(err) => ctx.report(err),
            }
        }
    }
}

fn validate_assignment_target(
    target: &Expression,
    func: Option<&FunctionSymbol>,
    init: Option<&InitializerSymbol>,
) -> Vec<AssignmentError> {
    let mut out = Vec::new();
    match &target.kind {
        ExprKind::LocalRef(local_id) => {
            if !target.is_mutable() {
                let name = get_local_name(*local_id, func, init).unwrap_or_else(|| "<unknown>".to_string());
                out.push(AssignmentError::ImmutableVar(CannotAssignToImmutableError { span: target.span.clone(), variable_name: name }));
            }
        }
        ExprKind::FieldAccess { object, field } => {
            let is_self_in_init = init.is_some() && is_self_expr(object);
            if !is_self_in_init && !target.is_mutable() {
                out.push(AssignmentError::ImmutableField(CannotAssignToImmutableFieldError { span: target.span.clone(), field_name: field.clone() }));
            }
        }
        ExprKind::TupleIndex { tuple: _, index } => {
            if !target.is_mutable() {
                out.push(AssignmentError::ImmutableField(CannotAssignToImmutableFieldError { span: target.span.clone(), field_name: format!("{}", index) }));
            }
        }
        // Invalid targets
        ExprKind::Literal(_)
        | ExprKind::Array(_)
        | ExprKind::Tuple(_)
        | ExprKind::Grouping(_)
        | ExprKind::Call { .. }
        | ExprKind::PrimitiveMethodCall { .. }
        | ExprKind::ImplicitStructInit { .. }
        | ExprKind::MethodRef { .. }
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::Assignment { .. }
        | ExprKind::If { .. }
        | ExprKind::While { .. }
        | ExprKind::Loop { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Return { .. }
        | ExprKind::Error => {
            out.push(AssignmentError::InvalidTarget(CannotAssignToExpressionError { span: target.span.clone() }));
        }
    }
    out
}

fn get_local_name(id: LocalId, func: Option<&FunctionSymbol>, init: Option<&InitializerSymbol>) -> Option<String> {
    if let Some(func) = func { func.get_local(id).map(|l| l.name().to_string()) }
    else if let Some(init) = init { init.get_local(id).map(|l| l.name().to_string()) }
    else { None }
}

fn is_self_expr(expr: &Expression) -> bool {
    match &expr.kind { ExprKind::LocalRef(local_id) => local_id.index() == 0, _ => false }
}

enum AssignmentError {
    ImmutableVar(CannotAssignToImmutableError),
    ImmutableField(CannotAssignToImmutableFieldError),
    InvalidTarget(CannotAssignToExpressionError),
}

 

pub mod diagnostics;
