//! Struct instantiation errors.
//!
//! Errors related to struct initialization, both implicit memberwise init
//! and explicit initializer calls.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

use super::call::OverloadDescription;

/// Error when implicit struct init has wrong number of arguments.
pub struct ImplicitInitArityError {
    /// Span of the instantiation expression
    pub span: Span,
    /// The struct name
    pub struct_name: String,
    /// Expected number of arguments (number of fields)
    pub expected: usize,
    /// Provided number of arguments
    pub provided: usize,
    /// Field names for reference
    pub field_names: Vec<String>,
}

impl IntoDiagnostic for ImplicitInitArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let fields_list = self.field_names.join(", ");
        Diagnostic::error()
            .with_message(format!(
                "struct '{}' has {} field(s), but {} argument(s) were provided",
                self.struct_name, self.expected, self.provided
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("expected {} argument(s)", self.expected)),
            ])
            .with_notes(vec![
                format!("fields: {}", fields_list),
                "implicit memberwise init requires an argument for each field in declaration order"
                    .to_string(),
            ])
    }
}

/// Error when implicit struct init has wrong label for a field.
pub struct ImplicitInitLabelError {
    /// Span of the instantiation expression
    pub span: Span,
    /// The struct name
    pub struct_name: String,
    /// Index of the mismatched argument (0-based)
    pub arg_index: usize,
    /// The label that was provided (None if unlabeled)
    pub provided_label: Option<String>,
    /// The expected label (field name)
    pub expected_label: String,
}

impl IntoDiagnostic for ImplicitInitLabelError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided_desc = match &self.provided_label {
            Some(label) => format!("'{}'", label),
            None => "unlabeled".to_string(),
        };

        Diagnostic::error()
            .with_message(format!(
                "argument {} has {} label, but expected '{}'",
                self.arg_index + 1,
                provided_desc,
                self.expected_label
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("expected label '{}'", self.expected_label)),
            ])
            .with_notes(vec![format!(
                "struct '{}' requires labeled arguments matching field names in declaration order",
                self.struct_name
            )])
    }
}

/// Error when no matching initializer is found for explicit init.
pub struct NoMatchingInitializerError {
    /// Span of the instantiation expression
    pub span: Span,
    /// The struct name
    pub struct_name: String,
    /// The argument labels provided (None = unlabeled)
    pub provided_labels: Vec<Option<String>>,
    /// Number of arguments provided
    pub provided_arity: usize,
    /// Available initializer overloads
    pub available_initializers: Vec<OverloadDescription>,
}

impl IntoDiagnostic for NoMatchingInitializerError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided = super::call::format_argument_labels(&self.provided_labels);

        let mut labels = vec![
            Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                "no matching initializer for {} argument(s) with labels {}",
                self.provided_arity, provided
            )),
        ];

        // Add secondary labels for available initializers
        for init in &self.available_initializers {
            if let (Some(span), Some(def_file_id)) =
                (&init.definition_span, init.definition_file_id)
            {
                labels.push(
                    Label::secondary(def_file_id, span.range())
                        .with_message(format!("candidate: {}", init.display())),
                );
            }
        }

        let mut notes = vec![];
        if !self.available_initializers.is_empty() {
            notes.push(format!(
                "available initializers for '{}':",
                self.struct_name
            ));
            for init in &self.available_initializers {
                notes.push(format!("  - {}", init.display()));
            }
        } else {
            notes.push(format!("struct '{}' has no initializers", self.struct_name));
        }

        Diagnostic::error()
            .with_message(format!(
                "no matching initializer for struct '{}'",
                self.struct_name
            ))
            .with_labels(labels)
            .with_notes(notes)
    }
}

/// Error when trying to use implicit init but a field is not visible.
pub struct FieldNotVisibleForInitError {
    /// Span of the instantiation expression
    pub span: Span,
    /// The struct name
    pub struct_name: String,
    /// The field that is not visible
    pub field_name: String,
    /// The field's visibility
    pub field_visibility: String,
}

impl IntoDiagnostic for FieldNotVisibleForInitError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "cannot use implicit initializer for '{}': field '{}' is {}",
                self.struct_name, self.field_name, self.field_visibility
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("implicit init not available"),
            ])
            .with_notes(vec![
                format!("field '{}' is not visible from this scope", self.field_name),
                "consider adding a public initializer to the struct".to_string(),
            ])
    }
}

/// Error when explicit init exists, suppressing implicit init.
pub struct ExplicitInitSuppressesImplicitError {
    /// Span of the instantiation expression
    pub span: Span,
    /// The struct name
    pub struct_name: String,
    /// The argument labels provided
    pub provided_labels: Vec<Option<String>>,
    /// Number of arguments provided
    pub provided_arity: usize,
    /// Available explicit initializers
    pub available_initializers: Vec<OverloadDescription>,
}

impl IntoDiagnostic for ExplicitInitSuppressesImplicitError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let provided = super::call::format_argument_labels(&self.provided_labels);

        let mut notes = vec![format!(
            "struct '{}' has explicit initializers, so implicit memberwise init is not available",
            self.struct_name
        )];

        if !self.available_initializers.is_empty() {
            notes.push("available initializers:".to_string());
            for init in &self.available_initializers {
                notes.push(format!("  - {}", init.display()));
            }
        }

        Diagnostic::error()
            .with_message(format!(
                "no matching initializer for struct '{}' with {} argument(s)",
                self.struct_name, self.provided_arity
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("no initializer matches labels {}", provided)),
            ])
            .with_notes(notes)
    }
}
