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
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("expected `{}`", expected)),
                ]),

            InferenceError::OccursCheck { var, ty, span } => Diagnostic::error()
                .with_message("infinite type detected")
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("type variable {:?} occurs in `{}`", var, ty)),
                ]),

            InferenceError::ConformanceFailure {
                ty,
                protocol_name,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "type `{}` does not conform to protocol `{}`",
                    ty, protocol_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("conformance required here"),
                ]),

            InferenceError::MemberNotFound {
                receiver,
                member,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "member not found: `{}` on type `{}`",
                    member, receiver
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("`{}` has no member `{}`", receiver, member)),
                ]),

            InferenceError::AssociatedTypeNotFound {
                container,
                assoc_name,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "associated type `{}` not found on `{}`",
                    assoc_name, container
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("associated type not found"),
                ]),

            InferenceError::Ambiguous { unresolved } => {
                let labels: Vec<_> = unresolved
                    .iter()
                    .map(|(_, span)| {
                        Label::primary(span.file_id, span.range())
                            .with_message("cannot infer type")
                    })
                    .collect();
                Diagnostic::error()
                    .with_message(format!(
                        "could not infer type for {} placeholder(s)",
                        unresolved.len()
                    ))
                    .with_labels(labels)
                    .with_notes(vec![
                        "try adding explicit type annotations to help the compiler".to_string(),
                    ])
            }

            InferenceError::Internal { message } => {
                Diagnostic::error().with_message(format!("internal inference error: {}", message))
            }

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
                    Label::primary(span.file_id, span.range()).with_message("used here"),
                ]),

            InferenceError::NoMatchingOverload {
                name,
                receiver_ty,
                provided_labels,
                expected_labels,
                span,
            } => {
                let provided: Vec<_> = provided_labels
                    .iter()
                    .map(|l| l.as_deref().unwrap_or("_"))
                    .collect();
                let expected: Vec<_> = expected_labels
                    .iter()
                    .map(|l| l.as_deref().unwrap_or("_"))
                    .collect();
                Diagnostic::error()
                    .with_message(format!("no matching overload for '{}'", name))
                    .with_labels(vec![
                        Label::primary(span.file_id, span.range()).with_message(format!(
                            "provided ({}), expected ({})",
                            provided.join(", "),
                            expected.join(", ")
                        )),
                    ])
                    .with_notes(vec![format!("on type `{}`", receiver_ty)])
            }

            InferenceError::CannotInferEnumType { member_name, span } => Diagnostic::error()
                .with_message(format!(
                    "cannot infer enum type for shorthand '.{}'",
                    member_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("type cannot be inferred from context"),
                ])
                .with_notes(vec![
                    "add a type annotation or use the full type path (e.g., `EnumType.Case`)"
                        .to_string(),
                ]),

            InferenceError::UnknownStructField {
                struct_name,
                field_name,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "struct `{}` has no field `{}`",
                    struct_name, field_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range()).with_message("unknown field"),
                ]),

            InferenceError::MissingStructFields {
                struct_name,
                missing_fields,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "pattern does not mention fields {} of `{}`",
                    missing_fields.join(", "),
                    struct_name
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("missing fields in pattern"),
                ])
                .with_notes(vec!["use `..` to ignore the remaining fields".to_string()]),

            InferenceError::UnknownEnumCase {
                enum_name,
                case_name,
                span,
            } => Diagnostic::error()
                .with_message(format!("enum `{}` has no case `{}`", enum_name, case_name))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("`{}` is not a case of `{}`", case_name, enum_name)),
                ]),

            InferenceError::TupleArityMismatch {
                expected,
                found,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "tuple pattern arity mismatch: expected {} elements, found {}",
                    expected, found
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message(format!("expected {} elements", expected)),
                ]),

            InferenceError::PrimitiveMethodNotCalled {
                method_name,
                receiver_type,
                span,
            } => Diagnostic::error()
                .with_message(format!(
                    "primitive method '{}' on '{}' must be called",
                    method_name, receiver_type
                ))
                .with_labels(vec![
                    Label::primary(span.file_id, span.range())
                        .with_message("add () to call this method"),
                ]),
        }
    }
}
