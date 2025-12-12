//! Type checking validator
//!
//! This validator performs type checking across the language:
//! - Return type checking (return expr matches function's declared return type)
//! - Assignment type checking (value type matches target type)
//! - Variable binding type checking (initializer matches declared type)
//! - Call argument type checking (argument types match parameter types)
//! - If/while condition checking (must be Bool)
//! - If branch type checking (branches must match when used as expression)
//! - Array element type checking (all elements must have same type)

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{compute_block_type, CallArgument, ElseBranch, ExprKind, Expression};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::Symbol;

use kestrel_semantic_model::{SemanticModel, SymbolFor};

use crate::body_resolver::format_type;
use crate::diagnostics::{
    ArrayElementTypeMismatchError, BranchTypeMismatchError, ConditionNotBoolError,
    TypeMismatchError,
};
use crate::validation::{BodyContext, Validator};
use super::type_assignability::is_assignable_with_constraints;

/// Validator for type checking
pub struct TypeCheckValidator;

impl TypeCheckValidator {
    const NAME: &'static str = "type_check";

    pub fn new() -> Self {
        Self
    }

    /// Check if a type is assignable to another, considering where clause constraints
    fn is_assignable(&self, from: &Ty, to: &Ty, ctx: &BodyContext<'_>) -> bool {
        let context_id = ctx.container.metadata().id();
        is_assignable_with_constraints(from, to, ctx.model, context_id)
    }

    /// Get the return type of the containing function/initializer
    fn get_return_type(&self, ctx: &BodyContext<'_>) -> Option<Ty> {
        // Try to get CallableBehavior from the container
        let behaviors = ctx.container.metadata().behaviors();
        for b in behaviors.iter() {
            if matches!(b.kind(), KestrelBehaviorKind::Callable) {
                if let Some(callable) = b.as_ref().downcast_ref::<CallableBehavior>() {
                    return Some(callable.return_type().clone());
                }
            }
        }
        None
    }

    /// Get the parameters of a callable from a call expression
    fn get_callable_params(&self, expr: &Expression, ctx: &BodyContext<'_>) -> Option<Vec<Ty>> {
        match &expr.kind {
            ExprKind::Call { callee, substitutions, .. } => {
                // Get the callable from the callee
                match &callee.kind {
                    ExprKind::SymbolRef(symbol_id) => {
                        let symbol = ctx.model.query(SymbolFor { id: *symbol_id })?;
                        let behaviors = symbol.metadata().behaviors();
                        for b in behaviors.iter() {
                            if matches!(b.kind(), KestrelBehaviorKind::Callable) {
                                if let Some(callable) =
                                    b.as_ref().downcast_ref::<CallableBehavior>()
                                {
                                    // Apply type argument substitutions to parameter types
                                    return Some(
                                        callable.parameters().iter().map(|p| {
                                            p.ty.apply_substitutions(substitutions)
                                        }).collect(),
                                    );
                                }
                            }
                        }
                    }
                    ExprKind::MethodRef { candidates, receiver, .. } => {
                        // Get first matching candidate's parameters
                        for &id in candidates {
                            if let Some(symbol) = ctx.model.query(SymbolFor { id }) {
                                let behaviors = symbol.metadata().behaviors();
                                for b in behaviors.iter() {
                                    if matches!(b.kind(), KestrelBehaviorKind::Callable) {
                                        if let Some(callable) =
                                            b.as_ref().downcast_ref::<CallableBehavior>()
                                        {
                                            // Apply type argument substitutions to parameter types
                                            // Also substitute Self with receiver type for method calls
                                            return Some(
                                                callable
                                                    .parameters()
                                                    .iter()
                                                    .map(|p| {
                                                        let ty = p.ty.apply_substitutions(substitutions);
                                                        substitute_self_type(&ty, &receiver.ty)
                                                    })
                                                    .collect(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            ExprKind::ImplicitStructInit { .. } => {
                // For implicit struct init, get field types from the struct type
                // and apply any substitutions from the instantiated type
                if let Some((struct_sym, substitutions)) = expr.ty.as_struct_with_subs() {
                    let fields: Vec<Ty> = struct_sym
                        .metadata()
                        .children()
                        .into_iter()
                        .filter(|c| {
                            c.metadata().kind()
                                == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Field
                        })
                        .filter_map(|f| {
                            let behaviors = f.metadata().behaviors();
                            for b in behaviors.iter() {
                                if matches!(
                                    b.kind(),
                                    KestrelBehaviorKind::Typed
                                ) {
                                    if let Some(typed) = b
                                        .as_ref()
                                        .downcast_ref::<kestrel_semantic_tree::behavior::typed::TypedBehavior>(
                                        )
                                    {
                                        // Apply substitutions to get the concrete field type
                                        let field_ty = typed.ty().apply_substitutions(substitutions);
                                        return Some(field_ty);
                                    }
                                }
                            }
                            None
                        })
                        .collect();
                    return Some(fields);
                }
            }
            _ => {}
        }
        None
    }

    /// Check a return expression
    fn check_return(&self, value: Option<&Expression>, expr: &Expression, ctx: &BodyContext<'_>) {
        let Some(expected_ty) = self.get_return_type(ctx) else {
            return;
        };

        // In initializers, bare return is allowed (implicitly returns self)
        let is_initializer = ctx.container.metadata().kind()
            == kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Initializer;

        match value {
            Some(value_expr) => {
                // Check that return value matches expected return type
                if !self.is_assignable(&value_expr.ty, &expected_ty, ctx) {
                    ctx.diagnostics().get().throw(
                        TypeMismatchError {
                            span: value_expr.span.clone(),
                            expected: format_type(&expected_ty),
                            found: format_type(&value_expr.ty),
                            context: "return value".to_string(),
                        });
                }
            }
            None => {
                // Bare `return` - function must return Unit, initializers allowed
                if !expected_ty.is_unit() && !is_initializer {
                    ctx.diagnostics().get().throw(
                        TypeMismatchError {
                            span: expr.span.clone(),
                            expected: format_type(&expected_ty),
                            found: "()".to_string(),
                            context: "return value".to_string(),
                        });
                }
            }
        }
    }

    /// Check an assignment expression
    fn check_assignment(
        &self,
        target: &Expression,
        value: &Expression,
        ctx: &BodyContext<'_>,
    ) {
        if !self.is_assignable(&value.ty, &target.ty, ctx) {
            ctx.diagnostics().get().throw(
                TypeMismatchError {
                    span: value.span.clone(),
                    expected: format_type(&target.ty),
                    found: format_type(&value.ty),
                    context: "assignment".to_string(),
                });
        }
    }

    /// Check an if expression's condition
    fn check_if_condition(&self, condition: &Expression, ctx: &BodyContext<'_>) {
        if !condition.ty.is_bool() && !condition.ty.is_error() {
            ctx.diagnostics().get().throw(
                ConditionNotBoolError {
                    span: condition.span.clone(),
                    found: format_type(&condition.ty),
                    condition_kind: "if",
                });
        }
    }

    /// Check an if expression's branch types
    fn check_if_branches(
        &self,
        then_branch: &[Statement],
        then_value: Option<&Expression>,
        else_branch: Option<&ElseBranch>,
        expr: &Expression,
        ctx: &BodyContext<'_>,
    ) {
        // Only check if there's an else branch (otherwise type is Unit)
        let Some(else_br) = else_branch else {
            return;
        };

        // Get the then branch type using the same logic as Expression::if_expr
        let then_ty = compute_block_type(then_branch, then_value, &expr.span);

        // Get the else branch type
        let else_ty = else_br.ty(&expr.span);

        // Skip checking if either branch has Never type (already handled by join)
        if then_ty.is_never() || else_ty.is_never() {
            return;
        }

        // Skip checking if either branch has Error type
        if then_ty.is_error() || else_ty.is_error() {
            return;
        }

        // Check that branches have compatible types
        if !self.is_assignable(&then_ty, &else_ty, ctx) {
            let then_span = then_value
                .map(|v| v.span.clone())
                .unwrap_or_else(|| expr.span.clone());

            let else_span = match else_br {
                ElseBranch::Block { value: Some(v), .. } => v.span.clone(),
                ElseBranch::Block { value: None, .. } => expr.span.clone(),
                ElseBranch::ElseIf(if_expr) => if_expr.span.clone(),
            };

            ctx.diagnostics().get().throw(
                BranchTypeMismatchError {
                    if_span: expr.span.clone(),
                    then_span,
                    else_span,
                    then_type: format_type(&then_ty),
                    else_type: format_type(&else_ty),
                });
        }
    }

    /// Check a while loop's condition
    fn check_while_condition(&self, condition: &Expression, ctx: &BodyContext<'_>) {
        if !condition.ty.is_bool() && !condition.ty.is_error() {
            ctx.diagnostics().get().throw(
                ConditionNotBoolError {
                    span: condition.span.clone(),
                    found: format_type(&condition.ty),
                    condition_kind: "while",
                });
        }
    }

    /// Check call argument types
    fn check_call_arguments(
        &self,
        expr: &Expression,
        arguments: &[CallArgument],
        ctx: &BodyContext<'_>,
    ) {
        let Some(param_types) = self.get_callable_params(expr, ctx) else {
            return;
        };

        // Check each argument against its parameter type
        for (i, (arg, param_ty)) in arguments.iter().zip(param_types.iter()).enumerate() {
            if !self.is_assignable(&arg.value.ty, param_ty, ctx) {
                let context = if let Some(ref label) = arg.label {
                    format!("argument '{}'", label)
                } else {
                    format!("argument {}", i + 1)
                };

                ctx.diagnostics().get().throw(
                    TypeMismatchError {
                        span: arg.value.span.clone(),
                        expected: format_type(param_ty),
                        found: format_type(&arg.value.ty),
                        context,
                    });
            }
        }
    }

    /// Check array element types
    fn check_array_elements(&self, elements: &[Expression], expr: &Expression, ctx: &BodyContext<'_>) {
        if elements.is_empty() {
            return;
        }

        let first = &elements[0];
        let expected_ty = &first.ty;

        // Skip if first element is error
        if expected_ty.is_error() {
            return;
        }

        for (i, elem) in elements.iter().enumerate().skip(1) {
            // Skip error elements
            if elem.ty.is_error() {
                continue;
            }

            if !self.is_assignable(&elem.ty, expected_ty, ctx) {
                ctx.diagnostics().get().throw(
                    ArrayElementTypeMismatchError {
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

impl Default for TypeCheckValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for TypeCheckValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &crate::validation::SymbolContext<'_>) {
        use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;

        // Check yield expression (implicit return) type for functions
        let kind = ctx.symbol.metadata().kind();
        if !matches!(
            kind,
            kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Function
                | kestrel_semantic_tree::symbol::kind::KestrelSymbolKind::Initializer
        ) {
            return;
        }

        // Get the callable behavior for return type
        let behaviors = ctx.symbol.metadata().behaviors();
        let callable = behaviors.iter().find_map(|b| {
            if matches!(b.kind(), KestrelBehaviorKind::Callable) {
                b.as_ref().downcast_ref::<CallableBehavior>()
            } else {
                None
            }
        });

        let Some(callable) = callable else {
            return;
        };
        let expected_ty = callable.return_type().clone();

        // Get the executable behavior for yield expression
        let executable = behaviors.iter().find_map(|b| {
            if matches!(b.kind(), KestrelBehaviorKind::Executable) {
                b.as_ref().downcast_ref::<ExecutableBehavior>()
            } else {
                None
            }
        });

        let Some(executable) = executable else {
            return;
        };

        // Check yield expression type
        // Skip for Unit-returning functions - the expression result is just discarded
        if let Some(yield_expr) = executable.body().yield_expr() {
            // Unit functions can have any expression (result discarded)
            // Non-unit functions must return the correct type
            let context_id = ctx.symbol.metadata().id();
            if !expected_ty.is_unit() && !is_assignable_with_constraints(&yield_expr.ty, &expected_ty, ctx.model, context_id) {
                ctx.diagnostics().get().throw(
                    TypeMismatchError {
                        span: yield_expr.span.clone(),
                        expected: format_type(&expected_ty),
                        found: format_type(&yield_expr.ty),
                        context: "return value".to_string(),
                    });
            }
        }
    }

    fn validate_expression(&self, expr: &Expression, ctx: &BodyContext<'_>) {
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
            ExprKind::Call { arguments, .. } => {
                self.check_call_arguments(expr, arguments, ctx);
            }
            ExprKind::ImplicitStructInit { arguments, .. } => {
                self.check_call_arguments(expr, arguments, ctx);
            }
            ExprKind::Array(elements) => {
                self.check_array_elements(elements, expr, ctx);
            }
            _ => {}
        }
    }

    fn validate_statement(&self, stmt: &Statement, ctx: &BodyContext<'_>) {
        // Check variable binding type
        if let StatementKind::Binding { pattern, value: Some(value) } = &stmt.kind {
            // Get the declared type from the pattern
            let declared_ty = &pattern.ty;

            // Skip if no declared type or if it's a type variable
            if declared_ty.is_type_var() || declared_ty.is_error() {
                return;
            }

            // Check that the value type matches the declared type
            if !self.is_assignable(&value.ty, declared_ty, ctx) {
                ctx.diagnostics().get().throw(
                    TypeMismatchError {
                        span: value.span.clone(),
                        expected: format_type(declared_ty),
                        found: format_type(&value.ty),
                        context: "variable binding".to_string(),
                    });
            }
        }
    }
}

/// Substitute SelfType with a concrete type in a type expression.
/// This is used for method calls where Self should be replaced with the receiver type.
fn substitute_self_type(ty: &Ty, replacement: &Ty) -> Ty {
    use kestrel_semantic_tree::ty::TyKind;

    match ty.kind() {
        TyKind::SelfType => replacement.clone(),
        TyKind::Tuple(elements) => {
            let new_elements: Vec<Ty> = elements.iter()
                .map(|e| substitute_self_type(e, replacement))
                .collect();
            Ty::tuple(new_elements, ty.span().clone())
        }
        TyKind::Array(element) => {
            let new_element = substitute_self_type(element, replacement);
            Ty::array(new_element, ty.span().clone())
        }
        TyKind::Function { params, return_type } => {
            let new_params: Vec<Ty> = params.iter()
                .map(|p| substitute_self_type(p, replacement))
                .collect();
            let new_return = substitute_self_type(return_type, replacement);
            Ty::function(new_params, new_return, ty.span().clone())
        }
        // For other types (primitives, structs, etc.), return as-is
        _ => ty.clone(),
    }
}
