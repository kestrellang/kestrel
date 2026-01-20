//! Error types for the lowering pass.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Errors that can occur during lowering.
#[derive(Debug, Clone)]
pub enum LoweringError {
    /// An unsupported language construct was encountered.
    UnsupportedConstruct {
        /// Description of the unsupported construct.
        construct: String,
        /// Source location.
        span: Span,
    },

    /// A function is missing its body.
    MissingFunctionBody {
        /// The function name.
        name: String,
        /// Source location.
        span: Span,
    },

    /// A type could not be lowered.
    UnsupportedType {
        /// Description of the type.
        type_desc: String,
        /// Source location.
        span: Span,
    },

    /// An expression kind is not yet implemented.
    UnsupportedExpression {
        /// Description of the expression kind.
        expr_kind: String,
        /// Source location.
        span: Span,
    },

    /// A statement kind is not yet implemented.
    UnsupportedStatement {
        /// Description of the statement kind.
        stmt_kind: String,
        /// Source location.
        span: Span,
    },

    /// A pattern kind is not yet implemented.
    UnsupportedPattern {
        /// Description of the pattern kind.
        pattern_kind: String,
        /// Source location.
        span: Span,
    },

    /// An item kind is not yet implemented.
    UnsupportedItem {
        /// Description of the item kind.
        item_kind: String,
        /// Source location.
        span: Span,
    },

    /// Internal error during lowering.
    Internal {
        /// Description of the error.
        message: String,
        /// Source location if available.
        span: Option<Span>,
    },
}

impl LoweringError {
    /// Create an unsupported construct error.
    pub fn unsupported(construct: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedConstruct {
            construct: construct.into(),
            span,
        }
    }

    /// Create an unsupported expression error.
    pub fn unsupported_expr(expr_kind: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedExpression {
            expr_kind: expr_kind.into(),
            span,
        }
    }

    /// Create an unsupported statement error.
    pub fn unsupported_stmt(stmt_kind: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedStatement {
            stmt_kind: stmt_kind.into(),
            span,
        }
    }

    /// Create an unsupported pattern error.
    pub fn unsupported_pattern(pattern_kind: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedPattern {
            pattern_kind: pattern_kind.into(),
            span,
        }
    }

    /// Create an unsupported item error.
    pub fn unsupported_item(item_kind: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedItem {
            item_kind: item_kind.into(),
            span,
        }
    }

    /// Create an unsupported type error.
    pub fn unsupported_type(type_desc: impl Into<String>, span: Span) -> Self {
        LoweringError::UnsupportedType {
            type_desc: type_desc.into(),
            span,
        }
    }

    /// Create a missing function body error.
    pub fn missing_body(name: impl Into<String>, span: Span) -> Self {
        LoweringError::MissingFunctionBody {
            name: name.into(),
            span,
        }
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>, span: Option<Span>) -> Self {
        LoweringError::Internal {
            message: message.into(),
            span,
        }
    }
}

impl IntoDiagnostic for LoweringError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        match self {
            LoweringError::UnsupportedConstruct { construct, span } => Diagnostic::warning()
                .with_message(format!("unsupported construct: {}", construct))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this construct is not yet supported in MIR lowering"),
                ]),

            LoweringError::MissingFunctionBody { name, span } => Diagnostic::error()
                .with_message(format!("function '{}' is missing a body", name))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("expected a function body"),
                ]),

            LoweringError::UnsupportedType { type_desc, span } => Diagnostic::warning()
                .with_message(format!("unsupported type: {}", type_desc))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this type is not yet supported in MIR lowering"),
                ]),

            LoweringError::UnsupportedExpression { expr_kind, span } => Diagnostic::warning()
                .with_message(format!("unsupported expression: {}", expr_kind))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this expression kind is not yet supported in MIR lowering"),
                ]),

            LoweringError::UnsupportedStatement { stmt_kind, span } => Diagnostic::warning()
                .with_message(format!("unsupported statement: {}", stmt_kind))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this statement kind is not yet supported in MIR lowering"),
                ]),

            LoweringError::UnsupportedPattern { pattern_kind, span } => Diagnostic::warning()
                .with_message(format!("unsupported pattern: {}", pattern_kind))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this pattern kind is not yet supported in MIR lowering"),
                ]),

            LoweringError::UnsupportedItem { item_kind, span } => Diagnostic::warning()
                .with_message(format!("unsupported item: {}", item_kind))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("this item kind is not yet supported in MIR lowering"),
                ]),

            LoweringError::Internal { message, span } => {
                let diag =
                    Diagnostic::bug().with_message(format!("internal lowering error: {}", message));
                if let Some(span) = span {
                    diag.with_labels(vec![
                        Label::primary(span.file_id, span.range())
                            .with_message("error occurred here"),
                    ])
                } else {
                    diag
                }
            }
        }
    }
}
