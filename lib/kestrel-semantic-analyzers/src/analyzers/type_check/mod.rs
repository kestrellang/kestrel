use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::CallableParamTypesForCall;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::expr::{
    compute_block_type, CallArgument, ElseBranch, ExprKind, Expression,
};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;

mod diagnostics;
use crate::analyzers::type_assignability::is_assignable_with_constraints;
use diagnostics::{
    ArrayElementTypeMismatchError, BranchTypeMismatchError, ConditionNotBoolError,
    TypeMismatchError,
};

pub struct TypeCheckAnalyzer;

impl TypeCheckAnalyzer {
    pub fn new() -> Self {
        Self
    }
}
impl Default for TypeCheckAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeCheckAnalyzer {
    fn name(&self) -> &'static str {
        "type_check"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

        let kind = symbol.metadata().kind();
        if !matches!(
            kind,
            KestrelSymbolKind::Function | KestrelSymbolKind::Initializer
        ) {
            return;
        }

        let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() else {
            return;
        };
        let expected_ty = callable.return_type().clone();

        let Some(executable) = symbol.metadata().get_behavior::<ExecutableBehavior>() else {
            return;
        };

        if let Some(yield_expr) = executable.body().yield_expr() {
            let context_id = symbol.metadata().id();
            if !expected_ty.is_unit()
                && !is_assignable_with_constraints(
                    &yield_expr.ty,
                    &expected_ty,
                    ctx.model,
                    context_id,
                )
            {
                ctx.report(TypeMismatchError {
                    span: yield_expr.span.clone(),
                    expected: format_type(&expected_ty),
                    found: format_type(&yield_expr.ty),
                    context: "return value".to_string(),
                });
            }
        }
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        match &expr.kind {
            ExprKind::Return { value } => {
                self.check_return(value.as_ref().map(|v| v.as_ref()), expr, ctx);
            }
            ExprKind::Assignment { target, value } => {
                self.check_assignment(target, value, ctx);
            }
            ExprKind::If {
                condition,
                then_branch,
                then_value,
                else_branch,
            } => {
                self.check_if_condition(condition, ctx);
                self.check_if_branches(
                    then_branch,
                    then_value.as_ref().map(|v| v.as_ref()),
                    else_branch.as_ref(),
                    expr,
                    ctx,
                );
            }
            ExprKind::While { condition, .. } => {
                self.check_while_condition(condition, ctx);
            }
            ExprKind::Call { arguments, .. } | ExprKind::ImplicitStructInit { arguments, .. } => {
                self.check_call_arguments(expr, arguments, ctx);
            }
            ExprKind::Array(elements) => {
                self.check_array_elements(elements, expr, ctx);
            }
            _ => {}
        }
    }

    fn visit_statement(&mut self, stmt: &Statement, ctx: &mut AnalysisContext) {
        if let StatementKind::Binding {
            pattern,
            value: Some(value),
        } = &stmt.kind
        {
            let declared_ty = &pattern.ty;
            if declared_ty.is_type_var() || declared_ty.is_error() {
                return;
            }
            if !is_assignable_in_ctx(&value.ty, declared_ty, ctx) {
                ctx.report(TypeMismatchError {
                    span: value.span.clone(),
                    expected: format_type(declared_ty),
                    found: format_type(&value.ty),
                    context: "variable binding".to_string(),
                });
            }
        }
    }
}

use kestrel_semantic_tree::language::KestrelLanguage;

impl TypeCheckAnalyzer {
    fn is_assignable(&self, from: &Ty, to: &Ty, ctx: &AnalysisContext) -> bool {
        let context_id = ctx
            .current_symbol()
            .map(|s| s.metadata().id())
            .unwrap_or_else(|| ctx.model.root().metadata().id());
        is_assignable_with_constraints(from, to, ctx.model, context_id)
    }

    fn get_return_type(&self, ctx: &AnalysisContext) -> Option<Ty> {
        ctx.current_symbol().and_then(|container| {
            container
                .metadata()
                .get_behavior::<CallableBehavior>()
                .map(|callable| callable.return_type().clone())
        })
    }

    fn check_return(
        &self,
        value: Option<&Expression>,
        expr: &Expression,
        ctx: &mut AnalysisContext,
    ) {
        let Some(expected_ty) = self.get_return_type(ctx) else {
            return;
        };
        let is_initializer = ctx
            .current_symbol()
            .map(|s| {
                s.metadata().kind()
                    == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Initializer
            })
            .unwrap_or(false);
        match value {
            Some(value_expr) => {
                if !self.is_assignable(&value_expr.ty, &expected_ty, ctx) {
                    ctx.report(TypeMismatchError {
                        span: value_expr.span.clone(),
                        expected: format_type(&expected_ty),
                        found: format_type(&value_expr.ty),
                        context: "return value".to_string(),
                    });
                }
            }
            None => {
                if !expected_ty.is_unit() && !is_initializer {
                    ctx.report(TypeMismatchError {
                        span: expr.span.clone(),
                        expected: format_type(&expected_ty),
                        found: "()".to_string(),
                        context: "return value".to_string(),
                    });
                }
            }
        }
    }

    fn check_assignment(&self, target: &Expression, value: &Expression, ctx: &mut AnalysisContext) {
        if !self.is_assignable(&value.ty, &target.ty, ctx) {
            ctx.report(TypeMismatchError {
                span: value.span.clone(),
                expected: format_type(&target.ty),
                found: format_type(&value.ty),
                context: "assignment".to_string(),
            });
        }
    }

    fn check_if_condition(&self, condition: &Expression, ctx: &mut AnalysisContext) {
        if !condition.ty.is_bool() && !condition.ty.is_error() {
            ctx.report(ConditionNotBoolError {
                span: condition.span.clone(),
                found: format_type(&condition.ty),
                condition_kind: "if",
            });
        }
    }

    fn check_if_branches(
        &self,
        then_branch: &[Statement],
        then_value: Option<&Expression>,
        else_branch: Option<&ElseBranch>,
        expr: &Expression,
        ctx: &mut AnalysisContext,
    ) {
        let Some(else_br) = else_branch else {
            return;
        };
        let then_ty = compute_block_type(then_branch, then_value, &expr.span);
        let else_ty = else_br.ty(&expr.span);
        if then_ty.is_never() || else_ty.is_never() {
            return;
        }
        if then_ty.is_error() || else_ty.is_error() {
            return;
        }
        if !self.is_assignable(&then_ty, &else_ty, ctx) {
            let then_span = then_value
                .map(|v| v.span.clone())
                .unwrap_or_else(|| expr.span.clone());
            let else_span = match else_br {
                ElseBranch::Block { value: Some(v), .. } => v.span.clone(),
                ElseBranch::Block { value: None, .. } => expr.span.clone(),
                ElseBranch::ElseIf(if_expr) => if_expr.span.clone(),
            };
            ctx.report(BranchTypeMismatchError {
                if_span: expr.span.clone(),
                then_span,
                else_span,
                then_type: format_type(&then_ty),
                else_type: format_type(&else_ty),
            });
        }
    }

    fn check_while_condition(&self, condition: &Expression, ctx: &mut AnalysisContext) {
        if !condition.ty.is_bool() && !condition.ty.is_error() {
            ctx.report(ConditionNotBoolError {
                span: condition.span.clone(),
                found: format_type(&condition.ty),
                condition_kind: "while",
            });
        }
    }

    fn check_call_arguments(
        &self,
        expr: &Expression,
        arguments: &[CallArgument],
        ctx: &mut AnalysisContext,
    ) {
        let Some(param_types) = ctx.model.query(CallableParamTypesForCall { expr }) else {
            return;
        };
        for (i, (arg, param_ty)) in arguments.iter().zip(param_types.iter()).enumerate() {
            if !self.is_assignable(&arg.value.ty, param_ty, ctx) {
                let context = if let Some(ref label) = arg.label {
                    format!("argument '{}'", label)
                } else {
                    format!("argument {}", i + 1)
                };
                ctx.report(TypeMismatchError {
                    span: arg.value.span.clone(),
                    expected: format_type(param_ty),
                    found: format_type(&arg.value.ty),
                    context,
                });
            }
        }
    }

    fn check_array_elements(
        &self,
        elements: &[Expression],
        expr: &Expression,
        ctx: &mut AnalysisContext,
    ) {
        if elements.is_empty() {
            return;
        }
        let first = &elements[0];
        let expected_ty = &first.ty;
        if expected_ty.is_error() {
            return;
        }
        for (i, elem) in elements.iter().enumerate().skip(1) {
            if elem.ty.is_error() {
                continue;
            }
            if !self.is_assignable(&elem.ty, expected_ty, ctx) {
                ctx.report(ArrayElementTypeMismatchError {
                    array_span: expr.span.clone(),
                    first_element_span: first.span.clone(),
                    element_span: elem.span.clone(),
                    element_index: i,
                    expected: format_type(expected_ty),
                    found: format_type(&elem.ty),
                });
            }
        }
    }
}

fn is_assignable_in_ctx(from: &Ty, to: &Ty, ctx: &AnalysisContext) -> bool {
    let context_id = ctx
        .current_symbol()
        .map(|s| s.metadata().id())
        .unwrap_or_else(|| ctx.model.root().metadata().id());
    is_assignable_with_constraints(from, to, ctx.model, context_id)
}

fn format_type(ty: &Ty) -> String {
    match ty.kind() {
        TyKind::Unit => "()".to_string(),
        TyKind::Never => "!".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Int(bits) => format!("{:?}", bits),
        TyKind::Float(bits) => format!("{:?}", bits),
        TyKind::Tuple(elements) => {
            let items: Vec<_> = elements.iter().map(format_type).collect();
            format!("({})", items.join(", "))
        }
        TyKind::Array(elem) => format!("[{}]", format_type(elem)),
        TyKind::Function {
            params,
            return_type,
        } => {
            let params_str: Vec<_> = params.iter().map(format_type).collect();
            format!(
                "({}) -> {}",
                params_str.join(", "),
                format_type(return_type)
            )
        }
        TyKind::Struct { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::Protocol { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::TypeParameter(param) => param.metadata().name().value.clone(),
        TyKind::TypeAlias { symbol, .. } => symbol.metadata().name().value.clone(),
        TyKind::AssociatedType { symbol, container } => match container {
            Some(container_ty) => format!(
                "{}.{}",
                format_type(container_ty),
                symbol.metadata().name().value
            ),
            None => symbol.metadata().name().value.clone(),
        },
        TyKind::SelfType => "Self".to_string(),
        TyKind::TypeVar(_) => "_".to_string(),
        TyKind::Error => "<error>".to_string(),
    }
}
