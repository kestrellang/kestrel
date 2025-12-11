//! Validator for assignment expressions
//!
//! This validator verifies that assignment expressions have valid targets:
//! - The target must be a mutable variable (`var`, not `let`)
//! - The target must be an lvalue (local variable, field access, etc.)
//! - For field access, the receiver must be mutable
//!
//! Type checking (RHS compatible with LHS) is handled elsewhere.

use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::local::LocalId;

use crate::diagnostics::{
    CannotAssignToExpressionError, CannotAssignToImmutableError, CannotAssignToImmutableFieldError,
};
use crate::validation::{BodyContext, Validator};

/// Validator for assignment target validity
pub struct AssignmentValidator;

impl AssignmentValidator {
    const NAME: &'static str = "assignment_validation";

    pub fn new() -> Self {
        Self
    }
}

impl Default for AssignmentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for AssignmentValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_expression(&self, expr: &Expression, ctx: &BodyContext<'_>) {
        // Only check assignment expressions
        let ExprKind::Assignment { target, .. } = &expr.kind else {
            return;
        };

        // Get function or initializer for local name lookup
        let func = ctx
            .container
            .as_ref()
            .downcast_ref::<FunctionSymbol>();
        let init = ctx
            .container
            .as_ref()
            .downcast_ref::<InitializerSymbol>();

        validate_assignment_target(target, func, init, ctx);
    }
}

/// Validate that an expression is a valid assignment target (lvalue)
fn validate_assignment_target(
    target: &Expression,
    func: Option<&FunctionSymbol>,
    init: Option<&InitializerSymbol>,
    ctx: &BodyContext<'_>,
) {
    match &target.kind {
        ExprKind::LocalRef(local_id) => {
            // Use the expression's mutable field directly
            if !target.is_mutable() {
                let name = get_local_name(*local_id, func, init)
                    .unwrap_or_else(|| "<unknown>".to_string());
                ctx.diagnostics().get().throw(
                    CannotAssignToImmutableError {
                        span: target.span.clone(),
                        variable_name: name,
                    });
            }
        }
        ExprKind::FieldAccess { object, field } => {
            // Special case: `self.field = value` in initializers is always allowed
            // (that's how fields get initialized, even `let` fields)
            let is_self_in_init = init.is_some() && is_self_expr(object);

            if !is_self_in_init && !target.is_mutable() {
                // Field access is immutable - either the field is `let` or the receiver is immutable
                ctx.diagnostics().get().throw(
                    CannotAssignToImmutableFieldError {
                        span: target.span.clone(),
                        field_name: field.clone(),
                    });
            }
        }
        ExprKind::TupleIndex { tuple: _, index } => {
            // Tuple element access - valid lvalue if tuple is mutable
            if !target.is_mutable() {
                ctx.diagnostics().get().throw(
                    CannotAssignToImmutableFieldError {
                        span: target.span.clone(),
                        field_name: format!("{}", index),
                    });
            }
        }
        // Invalid assignment targets - not lvalues at all
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
            ctx.diagnostics().get().throw(
                CannotAssignToExpressionError {
                    span: target.span.clone(),
                });
        }
    }
}

/// Get the name of a local by ID (for error messages)
fn get_local_name(
    id: LocalId,
    func: Option<&FunctionSymbol>,
    init: Option<&InitializerSymbol>,
) -> Option<String> {
    if let Some(func) = func {
        func.get_local(id).map(|l| l.name().to_string())
    } else if let Some(init) = init {
        init.get_local(id).map(|l| l.name().to_string())
    } else {
        None
    }
}

/// Check if an expression is a reference to `self`
///
/// This is used to allow `self.field = value` in initializers, where we're
/// initializing fields before `self` is fully constructed.
pub fn is_self_expr(expr: &Expression) -> bool {
    match &expr.kind {
        ExprKind::LocalRef(local_id) => {
            // The self parameter is always local 0 in initializers and methods
            local_id.index() == 0
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use kestrel_span::Span;
    use super::*;
    use kestrel_semantic_tree::ty::Ty;

    #[test]
    fn test_is_self_expr() {
        // Create a LocalRef to local 0 (self) - self is always mutable in initializers
        let self_expr = Expression::new(
            ExprKind::LocalRef(LocalId::new(0)),
            Ty::error(Span::from(0..1)),
            Span::from(0..4),
            true, // mutable
        );
        assert!(is_self_expr(&self_expr));

        // Create a LocalRef to local 1 (not self)
        let other_expr = Expression::new(
            ExprKind::LocalRef(LocalId::new(1)),
            Ty::error(Span::from(0..1)),
            Span::from(0..4),
            false,
        );
        assert!(!is_self_expr(&other_expr));

        // A literal is not self
        let literal = Expression::integer(42, Span::from(0..2));
        assert!(!is_self_expr(&literal));
    }

    #[test]
    fn test_is_self_expr_in_field_access() {
        // Create self.field expression
        let self_expr = Expression::new(
            ExprKind::LocalRef(LocalId::new(0)),
            Ty::error(Span::from(0..1)),
            Span::from(0..4),
            true, // self is mutable
        );
        let field_access = Expression::field_access(
            self_expr,
            "count".to_string(),
            true, // field is mutable (var)
            Ty::error(Span::from(0..1)),
            Span::from(0..10),
        );

        // The object inside the field access should be self
        if let ExprKind::FieldAccess { object, field: _ } = &field_access.kind {
            assert!(is_self_expr(object), "Expected object to be self");
        } else {
            panic!("Expected FieldAccess");
        }
        // Field access on mutable self with mutable field should be mutable
        assert!(field_access.is_mutable());
    }

    #[test]
    fn test_assignment_with_self_field() {
        // Create self.field = value expression
        let self_expr = Expression::new(
            ExprKind::LocalRef(LocalId::new(0)),
            Ty::error(Span::from(0..1)),
            Span::from(0..4),
            true, // self is mutable
        );
        let field_access = Expression::field_access(
            self_expr,
            "count".to_string(),
            true, // field is mutable (var)
            Ty::error(Span::from(0..1)),
            Span::from(0..10),
        );
        let value = Expression::integer(0, Span::from(11..12));
        let assignment = Expression::assignment(field_access, value, Span::from(0..12));

        // Check the structure
        if let ExprKind::Assignment { target, value: _ } = &assignment.kind {
            if let ExprKind::FieldAccess { object, field } = &target.kind {
                assert!(is_self_expr(object), "Expected object to be self");
                assert_eq!(field, "count");
            } else {
                panic!("Expected target to be FieldAccess");
            }
        } else {
            panic!("Expected Assignment");
        }
    }

    #[test]
    fn test_field_mutability_composition() {
        // Mutable parent + mutable field = mutable
        let mutable_parent = Expression::local_ref(LocalId::new(0), Ty::error(Span::from(0..1)), true, Span::from(0..4));
        let access1 = Expression::field_access(
            mutable_parent,
            "x".to_string(),
            true, // mutable field
            Ty::error(Span::from(0..1)),
            Span::from(0..6),
        );
        assert!(access1.is_mutable());

        // Mutable parent + immutable field = immutable
        let mutable_parent2 = Expression::local_ref(LocalId::new(0), Ty::error(Span::from(0..1)), true, Span::from(0..4));
        let access2 = Expression::field_access(
            mutable_parent2,
            "x".to_string(),
            false, // immutable field (let)
            Ty::error(Span::from(0..1)),
            Span::from(0..6),
        );
        assert!(!access2.is_mutable());

        // Immutable parent + mutable field = immutable
        let immutable_parent = Expression::local_ref(LocalId::new(0), Ty::error(Span::from(0..1)), false, Span::from(0..4));
        let access3 = Expression::field_access(
            immutable_parent,
            "x".to_string(),
            true, // mutable field
            Ty::error(Span::from(0..1)),
            Span::from(0..6),
        );
        assert!(!access3.is_mutable());
    }
}
