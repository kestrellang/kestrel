use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when static modifier is used in wrong context.
pub struct StaticInWrongContextError {
    pub span: Span,
    pub name: String,
    pub context: StaticContext,
}

/// The invalid context where static was used.
pub enum StaticContext {
    /// Static used at module level
    ModuleLevel,
}

impl IntoDiagnostic for StaticInWrongContextError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let context_msg = match self.context {
            StaticContext::ModuleLevel => "static is not allowed at module level",
        };

        Diagnostic::error()
            .with_message(format!("'{}' cannot be static in this context", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(context_msg)
            ])
    }
}

