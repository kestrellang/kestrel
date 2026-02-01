use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a computed property uses 'let' instead of 'var'.
pub struct ComputedPropertyMustBeVarError {
    pub span: Span,
}

impl IntoDiagnostic for ComputedPropertyMustBeVarError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("computed properties must use 'var'")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("computed property declared with 'let'"),
            ])
    }
}

/// Error when a property in global context uses the 'static' modifier.
pub struct GlobalPropertyStaticModifierError {
    pub span: Span,
    pub is_computed: bool,
}

impl IntoDiagnostic for GlobalPropertyStaticModifierError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let message = if self.is_computed {
            "computed properties in global context are already static"
        } else {
            "properties in global context are already static"
        };

        Diagnostic::error().with_message(message).with_labels(vec![
            Label::primary(self.span.file_id, self.span.range())
                .with_message("'static' modifier used in global context"),
        ])
    }
}

/// Error when an enum has a non-static stored field.
pub struct EnumStoredFieldError {
    pub span: Span,
}

impl IntoDiagnostic for EnumStoredFieldError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("enums cannot have stored fields")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("stored field declared here"),
            ])
    }
}

/// Error when a generic type has a static stored property.
pub struct GenericTypeStaticStoredPropertyError {
    pub span: Span,
    pub type_name: String,
}

impl IntoDiagnostic for GenericTypeStaticStoredPropertyError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("static stored properties not supported in generic types")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "static stored property in generic type '{}'",
                    self.type_name
                )),
            ])
    }
}
