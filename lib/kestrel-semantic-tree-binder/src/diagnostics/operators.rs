//! Operator-related errors.
//!
//! Errors for binary and unary operator resolution.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a binary operator is not supported on a type.
pub struct UnsupportedBinaryOperator {
    /// Span of the operator
    pub operator_span: Span,
    /// The operator symbol (e.g., "+", "-", "and")
    pub operator: String,
    /// Span of the left operand
    pub lhs_span: Span,
    /// String representation of the left operand type
    pub lhs_type: String,
    /// Span of the right operand
    pub rhs_span: Span,
    /// String representation of the right operand type
    pub rhs_type: String,
}

impl IntoDiagnostic for UnsupportedBinaryOperator {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "binary operator '{}' cannot be applied to types '{}' and '{}'",
                self.operator, self.lhs_type, self.rhs_type
            ))
            .with_labels(vec![
                Label::primary(self.operator_span.file_id, self.operator_span.range())
                    .with_message("unsupported operator"),
                Label::secondary(self.lhs_span.file_id, self.lhs_span.range())
                    .with_message(format!("has type '{}'", self.lhs_type)),
                Label::secondary(self.rhs_span.file_id, self.rhs_span.range())
                    .with_message(format!("has type '{}'", self.rhs_type)),
            ])
    }
}

/// Error when a unary operator is not supported on a type.
pub struct UnsupportedUnaryOperator {
    /// Span of the operator
    pub operator_span: Span,
    /// The operator symbol (e.g., "-", "not", "!")
    pub operator: String,
    /// Span of the operand
    pub operand_span: Span,
    /// String representation of the operand type
    pub operand_type: String,
}

impl IntoDiagnostic for UnsupportedUnaryOperator {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "unary operator '{}' cannot be applied to type '{}'",
                self.operator, self.operand_type
            ))
            .with_labels(vec![
                Label::primary(self.operator_span.file_id, self.operator_span.range())
                    .with_message("unsupported operator"),
                Label::secondary(self.operand_span.file_id, self.operand_span.range())
                    .with_message(format!("has type '{}'", self.operand_type)),
            ])
    }
}
