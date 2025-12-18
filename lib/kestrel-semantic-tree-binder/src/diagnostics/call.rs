//! Call-related errors.
//!
//! Errors related to function calls, method calls, and overload resolution.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// A description of an available overload for error messages.
#[derive(Debug, Clone)]
pub struct OverloadDescription {
    /// The function/method name
    pub name: String,
    /// The parameter labels (None = unlabeled positional parameter)
    pub labels: Vec<Option<String>>,
    /// The parameter type descriptions
    pub param_types: Vec<String>,
    /// The span where this overload is defined (for secondary labels)
    pub definition_span: Option<Span>,
    /// The file ID where this overload is defined
    pub definition_file_id: Option<usize>,
}

impl OverloadDescription {
    /// Format the overload signature for display.
    pub fn display(&self) -> String {
        let params: Vec<String> = self
            .labels
            .iter()
            .zip(self.param_types.iter())
            .map(|(label, ty)| match label {
                Some(l) => format!("{}: {}", l, ty),
                None => ty.clone(),
            })
            .collect();

        format!("{}({})", self.name, params.join(", "))
    }
}

/// Format argument labels for display in error messages.
pub fn format_argument_labels(labels: &[Option<String>]) -> String {
    if labels.is_empty() {
        return "()".to_string();
    }

    let formatted: Vec<String> = labels
        .iter()
        .map(|l| match l {
            Some(label) => format!("{}:", label),
            None => "_".to_string(),
        })
        .collect();

    format!("({})", formatted.join(", "))
}

/// Error when no overload matches the provided arguments.
pub struct NoMatchingOverloadError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// The function/method name being called
    pub name: String,
    /// The argument labels provided (None = unlabeled)
    pub provided_labels: Vec<Option<String>>,
    /// Number of arguments provided
    pub provided_arity: usize,
    /// Available overloads that didn't match
    pub available_overloads: Vec<OverloadDescription>,
}

impl IntoDiagnostic for NoMatchingOverloadError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided = format_argument_labels(&self.provided_labels);

        let mut labels = vec![
            Label::primary(self.call_span.file_id, self.call_span.range()).with_message(format!(
                "no matching overload for {} arguments with labels {}",
                self.provided_arity, provided
            )),
        ];

        // Add secondary labels for available overloads (if we have location info)
        for overload in &self.available_overloads {
            if let (Some(span), Some(def_file_id)) =
                (&overload.definition_span, overload.definition_file_id)
            {
                labels.push(
                    Label::secondary(def_file_id, span.range())
                        .with_message(format!("candidate: {}", overload.display())),
                );
            }
        }

        let mut notes = vec![];
        if !self.available_overloads.is_empty() {
            notes.push("available overloads:".to_string());
            for overload in &self.available_overloads {
                notes.push(format!("  - {}", overload.display()));
            }
        }

        Diagnostic::error()
            .with_message(format!(
                "no matching overload for '{}' with {} argument(s)",
                self.name, self.provided_arity
            ))
            .with_labels(labels)
            .with_notes(notes)
    }
}

/// Error when no method matches the provided arguments.
pub struct NoMatchingMethodError {
    /// Span of the entire call expression
    pub call_span: Span,
    /// The method name being called
    pub method_name: String,
    /// The receiver type
    pub receiver_type: String,
    /// The argument labels provided (None = unlabeled)
    pub provided_labels: Vec<Option<String>>,
    /// Number of arguments provided
    pub provided_arity: usize,
    /// Available method overloads that didn't match
    pub available_overloads: Vec<OverloadDescription>,
}

impl IntoDiagnostic for NoMatchingMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided = format_argument_labels(&self.provided_labels);

        let mut labels = vec![
            Label::primary(self.call_span.file_id, self.call_span.range()).with_message(format!(
                "no method '{}' with {} argument(s) and labels {}",
                self.method_name, self.provided_arity, provided
            )),
        ];

        // Add secondary labels for available overloads (if we have location info)
        for overload in &self.available_overloads {
            if let (Some(span), Some(def_file_id)) =
                (&overload.definition_span, overload.definition_file_id)
            {
                labels.push(
                    Label::secondary(def_file_id, span.range())
                        .with_message(format!("candidate: {}", overload.display())),
                );
            }
        }

        let mut notes = vec![];
        if !self.available_overloads.is_empty() {
            notes.push(format!("available methods on '{}':", self.receiver_type));
            for overload in &self.available_overloads {
                notes.push(format!("  - {}", overload.display()));
            }
        }

        Diagnostic::error()
            .with_message(format!(
                "no method '{}' on type '{}' matches the provided arguments",
                self.method_name, self.receiver_type
            ))
            .with_labels(labels)
            .with_notes(notes)
    }
}

/// Error when a primitive method is called with wrong number of arguments.
pub struct PrimitiveMethodArityError {
    /// Span of the call expression
    pub call_span: Span,
    /// The method name
    pub method_name: String,
    /// The receiver type
    pub receiver_type: String,
    /// Expected number of arguments
    pub expected_arity: usize,
    /// Provided number of arguments
    pub provided_arity: usize,
}

impl IntoDiagnostic for PrimitiveMethodArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "method '{}' on '{}' takes {} argument(s), but {} were provided",
                self.method_name, self.receiver_type, self.expected_arity, self.provided_arity
            ))
            .with_labels(vec![
                Label::primary(self.call_span.file_id, self.call_span.range())
                    .with_message(format!("expected {} argument(s)", self.expected_arity)),
            ])
    }
}

/// Error when trying to access a primitive method without calling it.
pub struct PrimitiveMethodNotCallableError {
    /// Span of the member access expression
    pub span: Span,
    /// The method name
    pub method_name: String,
    /// The receiver type
    pub receiver_type: String,
}

impl IntoDiagnostic for PrimitiveMethodNotCallableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "primitive method '{}' on '{}' must be called",
                self.method_name, self.receiver_type
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("add () to call this method"),
            ])
            .with_notes(vec![format!(
                "primitive methods cannot be used as first-class values; use {}.{}() instead",
                self.receiver_type, self.method_name
            )])
    }
}

/// Error when no method exists on a type.
pub struct NoSuchMethodError {
    /// Span of the method call
    pub call_span: Span,
    /// The method name being called
    pub method_name: String,
    /// The receiver type
    pub receiver_type: String,
}

impl IntoDiagnostic for NoSuchMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "no method '{}' on type '{}'",
                self.method_name, self.receiver_type
            ))
            .with_labels(vec![
                Label::primary(self.call_span.file_id, self.call_span.range())
                    .with_message("method not found"),
            ])
    }
}

/// Error when 'self' is used outside of an instance method.
pub struct SelfOutsideInstanceMethodError {
    /// Span of the 'self' reference
    pub span: Span,
    /// Context description (e.g., "static method", "free function")
    pub context: String,
}

impl IntoDiagnostic for SelfOutsideInstanceMethodError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("cannot use 'self' in {}", self.context))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("'self' is only available in instance methods"),
            ])
            .with_notes(vec![
                "'self' is implicitly defined only in non-static methods of structs and protocols"
                    .to_string(),
            ])
    }
}

/// Error when an undefined name is referenced.
pub struct UndefinedNameError {
    /// Span of the undefined name
    pub span: Span,
    /// The name that was not found
    pub name: String,
}

impl IntoDiagnostic for UndefinedNameError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("undefined name '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not found in this scope"),
            ])
    }
}

/// Error when calling an instance method on a type instead of an instance.
pub struct InstanceMethodOnTypeError {
    /// Span of the call expression
    pub span: Span,
    /// The type name
    pub type_name: String,
    /// The method name
    pub method_name: String,
}

impl IntoDiagnostic for InstanceMethodOnTypeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot call instance method '{}' on type '{}'",
                self.method_name, self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("instance method requires an instance"),
            ])
            .with_notes(vec![format!(
                "call this method on an instance of '{}', not the type itself",
                self.type_name
            )])
    }
}

/// Error when type arguments are provided to a non-generic expression (like a variable).
pub struct TypeArgsOnNonGenericError {
    /// Span of the type argument list
    pub span: Span,
    /// Description of what was being called (e.g., "variable", "expression")
    pub callee_description: String,
}

impl IntoDiagnostic for TypeArgsOnNonGenericError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type arguments cannot be applied to {}",
                self.callee_description
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type arguments not allowed here"),
            ])
            .with_notes(vec![
                "type arguments can only be applied to generic functions or types".to_string(),
            ])
    }
}

/// Error when an expression is not callable.
pub struct NonCallableError {
    /// Span of the call expression
    pub span: Span,
    /// String representation of the type that is not callable
    pub ty: String,
}

impl IntoDiagnostic for NonCallableError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("type `{}` is not callable", self.ty))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("this has type `{}`", self.ty)),
            ])
            .with_notes(vec![format!(
                "only functions and types can be called, but this expression has type `{}`",
                self.ty
            )])
    }
}

/// Error when a name is ambiguous (multiple candidates in scope).
pub struct AmbiguousNameError {
    /// Span of the ambiguous name
    pub span: Span,
    /// The name that was found to be ambiguous
    pub name: String,
    /// Number of candidates found
    pub candidate_count: usize,
}

impl IntoDiagnostic for AmbiguousNameError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("ambiguous name '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "{} symbols with this name in scope",
                    self.candidate_count
                )),
            ])
            .with_notes(vec![
                "Use a fully qualified path to disambiguate.".to_string(),
            ])
    }
}
