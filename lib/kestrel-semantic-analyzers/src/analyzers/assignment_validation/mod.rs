use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use diagnostics::{
    CannotAssignToExpressionError, CannotAssignToImmutableError, CannotAssignToImmutableFieldError,
};

use kestrel_semantic_model::LocalName;
use kestrel_semantic_tree::expr::{ExprKind, Expression};

pub struct AssignmentValidationAnalyzer;

impl AssignmentValidationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for AssignmentValidationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for AssignmentValidationAnalyzer {
    fn name(&self) -> &'static str {
        "assignment_validation"
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        let ExprKind::Assignment { target, .. } = &expr.kind else {
            return;
        };

        let container_id = ctx.current_symbol().map(|s| s.metadata().id());
        let is_initializer = ctx
            .current_symbol()
            .map(|s| {
                s.metadata().kind()
                    == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Initializer
            })
            .unwrap_or(false);

        let errors = validate_assignment_target(target, container_id, is_initializer, ctx);
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
    container_id: Option<semantic_tree::symbol::SymbolId>,
    is_initializer: bool,
    ctx: &AnalysisContext,
) -> Vec<AssignmentError> {
    let mut out = Vec::new();
    match &target.kind {
        ExprKind::LocalRef(local_id) => {
            if !target.is_mutable() {
                let name = container_id
                    .and_then(|container_id| {
                        ctx.model.query(LocalName {
                            container_id,
                            local_id: *local_id,
                        })
                    })
                    .unwrap_or_else(|| "<unknown>".to_string());
                out.push(AssignmentError::ImmutableVar(
                    CannotAssignToImmutableError {
                        span: target.span.clone(),
                        variable_name: name,
                    },
                ));
            }
        }
        ExprKind::FieldAccess { object, field } => {
            let is_self_in_init = is_initializer && is_self_expr(object);
            if !is_self_in_init && !target.is_mutable() {
                out.push(AssignmentError::ImmutableField(
                    CannotAssignToImmutableFieldError {
                        span: target.span.clone(),
                        field_name: field.clone(),
                    },
                ));
            }
        }
        ExprKind::TupleIndex { tuple: _, index } => {
            if !target.is_mutable() {
                out.push(AssignmentError::ImmutableField(
                    CannotAssignToImmutableFieldError {
                        span: target.span.clone(),
                        field_name: format!("{}", index),
                    },
                ));
            }
        }
        // Invalid targets
        ExprKind::Literal(_)
        | ExprKind::Array(_)
        | ExprKind::Tuple(_)
        | ExprKind::Grouping(_)
        | ExprKind::Call { .. }
        | ExprKind::PrimitiveMethodCall { .. }
        | ExprKind::DeferredMethodCall { .. }
        | ExprKind::ImplicitStructInit { .. }
        | ExprKind::MethodRef { .. }
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::ImplicitMemberAccess { .. }
        | ExprKind::Assignment { .. }
        | ExprKind::If { .. }
        | ExprKind::While { .. }
        | ExprKind::WhileLet { .. }
        | ExprKind::Loop { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::Return { .. }
        | ExprKind::Closure { .. }
        | ExprKind::Match { .. }
        | ExprKind::Block { .. }
        | ExprKind::Error => {
            out.push(AssignmentError::InvalidTarget(
                CannotAssignToExpressionError {
                    span: target.span.clone(),
                },
            ));
        }
    }
    out
}

fn is_self_expr(expr: &Expression) -> bool {
    match &expr.kind {
        ExprKind::LocalRef(local_id) => local_id.index() == 0,
        _ => false,
    }
}

enum AssignmentError {
    ImmutableVar(CannotAssignToImmutableError),
    ImmutableField(CannotAssignToImmutableFieldError),
    InvalidTarget(CannotAssignToExpressionError),
}

pub mod diagnostics;
