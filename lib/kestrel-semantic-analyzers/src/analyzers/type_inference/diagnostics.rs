//! Diagnostics for type inference errors.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_semantic_type_inference::InferenceError;

/// Wrapper for converting inference errors to diagnostics.
pub struct InferenceErrorDiagnostic(InferenceError);

impl From<InferenceError> for InferenceErrorDiagnostic {
    fn from(error: InferenceError) -> Self {
        Self(error)
    }
}

impl IntoDiagnostic for InferenceErrorDiagnostic {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        match &self.0 {
            InferenceError::TypeMismatch {
                expected,
                found,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "type mismatch: expected `{}`, found `{}`",
                    expected, found
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())
                    .with_message(format!("expected `{}`", expected))]),

            InferenceError::OccursCheck { var, ty, span } => Diagnostic::error()
                .with_message("infinite type detected")
                .with_labels(vec![Label::primary(span.file_id, span.range())
                    .with_message(format!("type variable {:?} occurs in `{}`", var, ty))]),

            InferenceError::ConformanceFailure {
                ty,
                protocol_name,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "type `{}` does not conform to protocol `{}`",
                    ty, protocol_name
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())
                    .with_message("conformance required here")]),

            InferenceError::MemberNotFound {
                receiver,
                member,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "no member `{}` found for type `{}`",
                    member, receiver
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())
                    .with_message(format!("`{}` has no member `{}`", receiver, member))]),

            InferenceError::AssociatedTypeNotFound {
                container,
                assoc_name,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "associated type `{}` not found on `{}`",
                    assoc_name, container
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())
                    .with_message("associated type not found")]),

            InferenceError::Ambiguous { unresolved } => Diagnostic::error()
                .with_message(format!(
                    "could not infer type for {} placeholder(s)",
                    unresolved.len()
                ))
                .with_notes(vec![
                    "try adding explicit type annotations to help the compiler".to_string()
                ]),

            InferenceError::Internal { message } => Diagnostic::error()
                .with_message(format!("internal inference error: {}", message)),

            InferenceError::ClosureArityMismatch {
                actual,
                expected,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "closure has {} parameters but {} expected",
                    actual, expected
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())]),

            InferenceError::ClosureReturnTypeMismatch {
                actual,
                expected,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "closure returns `{}` but `{}` expected",
                    actual, expected
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())]),

            InferenceError::ClosureParamTypeMismatch {
                index,
                actual,
                expected,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "closure parameter {} has type `{}` but `{}` expected",
                    index + 1,
                    actual,
                    expected
                ))
                .with_labels(vec![Label::primary(span.file_id, span.range())]),

            InferenceError::ItUsedWithWrongArity {
                expected_arity,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "`it` can only be used when closure has exactly 1 parameter, but {} expected",
                    expected_arity
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range()).with_message("used here")
                ]),
        }
    }
}
