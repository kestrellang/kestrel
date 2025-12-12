use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

#[derive(Clone, Debug)]
pub enum InitializerError {
    LetFieldAssignedTwice {
        span: Span,
        field_name: String,
    },
    FieldReadBeforeAssigned {
        span: Span,
        field_name: String,
    },
    SelfUsedBeforeFullyInitialized {
        span: Span,
        uninitialized: Vec<String>,
    },
    ReturnBeforeFullyInitialized {
        span: Span,
        uninitialized: Vec<String>,
    },
}

impl IntoDiagnostic for InitializerError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        match self {
            InitializerError::LetFieldAssignedTwice { span, field_name } => Diagnostic::error()
                .with_message(format!(
                    "cannot assign to 'let' field '{}' more than once",
                    field_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("second assignment here"),
                ]),
            InitializerError::FieldReadBeforeAssigned { span, field_name } => Diagnostic::error()
                .with_message(format!(
                    "cannot read field '{}' before it is initialized",
                    field_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range()).with_message("field read here"),
                ]),
            InitializerError::SelfUsedBeforeFullyInitialized {
                span,
                uninitialized,
            } => {
                let fields = uninitialized.join(", ");
                Diagnostic::error()
                    .with_message("cannot use 'self' before all fields are initialized")
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range()).with_message("self used here"),
                    ])
                    .with_notes(vec![format!("uninitialized fields: {}", fields)])
            }
            InitializerError::ReturnBeforeFullyInitialized {
                span,
                uninitialized,
            } => {
                let fields = uninitialized.join(", ");
                Diagnostic::error()
                    .with_message("cannot return before all fields are initialized")
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range()).with_message("return here"),
                    ])
                    .with_notes(vec![format!("uninitialized fields: {}", fields)])
            }
        }
    }
}

pub struct UninitializedFieldsError {
    pub span: Span,
    pub fields: String,
}

impl IntoDiagnostic for UninitializedFieldsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "initializer does not initialize all fields: {}",
                self.fields
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("in this initializer"),
            ])
    }
}
