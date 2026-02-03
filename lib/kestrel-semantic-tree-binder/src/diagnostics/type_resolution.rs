//! Type resolution errors.
//!
//! Errors related to resolving type paths and generic type instantiation.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when a type cannot be found in scope.
pub struct UnresolvedTypeError {
    pub span: Span,
    pub type_name: String,
}

impl IntoDiagnostic for UnresolvedTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot find type '{}' in this scope",
                self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message("not found"),
            ])
    }
}

/// Error when a type name is ambiguous (multiple candidates in scope).
pub struct AmbiguousTypeError {
    pub span: Span,
    pub type_name: String,
    pub candidate_count: usize,
}

impl IntoDiagnostic for AmbiguousTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("type '{}' is ambiguous", self.type_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "{} types with this name in scope",
                    self.candidate_count
                )),
            ])
            .with_notes(vec![
                "Use a fully qualified path to disambiguate.".to_string(),
            ])
    }
}

/// Error when a symbol is not a type.
pub struct NotATypeError {
    pub span: Span,
    pub name: String,
}

impl IntoDiagnostic for NotATypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("'{}' is not a type", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message("not a type"),
            ])
    }
}

/// Error when type arguments are provided to a non-generic type.
pub struct NotGenericError {
    pub span: Span,
    pub type_name: String,
}

impl IntoDiagnostic for NotGenericError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type '{}' does not accept type arguments",
                self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not a generic type"),
            ])
            .with_notes(vec![format!(
                "'{}' is not declared with type parameters",
                self.type_name
            )])
    }
}

/// Error when too few type arguments are provided.
pub struct TooFewTypeArgumentsError {
    pub span: Span,
    pub type_name: String,
    pub min_expected: usize,
    pub got: usize,
}

impl IntoDiagnostic for TooFewTypeArgumentsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("too few type arguments for '{}'", self.type_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "expected at least {}, found {}",
                    self.min_expected, self.got
                )),
            ])
    }
}

/// Error when too many type arguments are provided.
pub struct TooManyTypeArgumentsError {
    pub span: Span,
    pub type_name: String,
    pub max_expected: usize,
    pub got: usize,
}

impl IntoDiagnostic for TooManyTypeArgumentsError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("too many type arguments for '{}'", self.type_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "expected at most {}, found {}",
                    self.max_expected, self.got
                )),
            ])
    }
}

/// Error when type parameters are used in the wrong position in an extension.
pub struct TypeParameterWrongPositionError {
    pub span: Span,
    pub message: String,
}

impl IntoDiagnostic for TypeParameterWrongPositionError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(self.message.clone())
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type parameters must appear in their declared positions")
            ])
            .with_notes(vec![
                "Extensions cannot reorder type parameters. Use the same order as the type declaration.".to_string()
            ])
    }
}

/// Error when lang.ptr has wrong number of type arguments.
pub struct LangPtrArityError {
    pub span: Span,
    pub got: usize,
}

impl IntoDiagnostic for LangPtrArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let message = if self.got == 0 {
            "lang.ptr requires exactly 1 type argument".to_string()
        } else {
            format!(
                "too many type arguments for 'lang.ptr': expected 1, found {}",
                self.got
            )
        };
        Diagnostic::error().with_message(message).with_labels(vec![
            Label::primary(self.span.file_id, self.span.range())
                .with_message("type argument required"),
        ])
    }
}

/// Error when a type operator builtin is not defined.
///
/// This occurs when the syntax sugar (e.g., `[T]`, `T?`, `[K: V]`, `T throws E`)
/// cannot be resolved because the corresponding builtin type alias is not registered.
pub struct TypeOperatorNotDefinedError {
    pub span: Span,
    pub operator_name: String,
}

impl IntoDiagnostic for TypeOperatorNotDefinedError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "{} is not defined",
                self.operator_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type operator cannot be resolved"),
            ])
            .with_notes(vec![
                "Is the standard library imported?".to_string(),
            ])
    }
}

/// Error when a type operator symbol cannot be found in the registry.
///
/// This is an internal consistency error - the builtin was registered but
/// the symbol ID doesn't exist in the symbol registry.
pub struct TypeOperatorSymbolNotFoundError {
    pub span: Span,
    pub operator_name: String,
}

impl IntoDiagnostic for TypeOperatorSymbolNotFoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "internal error: {} symbol not found in registry",
                self.operator_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type operator symbol missing"),
            ])
            .with_notes(vec![
                "This is an internal compiler error. Please report this bug.".to_string(),
            ])
    }
}

/// Error when a type operator symbol has an unexpected type.
///
/// This is an internal consistency error - the symbol exists but is not
/// a TypeAliasSymbol as expected.
pub struct TypeOperatorInvalidSymbolError {
    pub span: Span,
    pub operator_name: String,
}

impl IntoDiagnostic for TypeOperatorInvalidSymbolError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "internal error: {} is not a type alias",
                self.operator_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type operator has wrong symbol type"),
            ])
            .with_notes(vec![
                "This is an internal compiler error. Please report this bug.".to_string(),
            ])
    }
}
