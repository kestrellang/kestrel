//! Error types for import resolution and semantic analysis

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Error when an imported module path cannot be resolved
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleNotFoundError {
    /// The full import path that failed (e.g., ["A", "B", "C"])
    pub path: Vec<String>,
    /// The segment index where resolution failed (0-based)
    pub failed_segment_index: usize,
    /// Span of the entire module path in import statement
    pub path_span: Span,
    /// Span of the specific segment that failed
    pub failed_segment_span: Span,
}

impl IntoDiagnostic for ModuleNotFoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let failed_segment = &self.path[self.failed_segment_index];

        let partial_path = if self.failed_segment_index == 0 {
            failed_segment.clone()
        } else {
            self.path[..=self.failed_segment_index].join(".")
        };

        Diagnostic::error()
            .with_message(format!("module '{}' not found", partial_path))
            .with_labels(vec![
                Label::primary(self.failed_segment_span.file_id, self.failed_segment_span.range())
                    .with_message(format!("no module named '{}'", failed_segment)),
                Label::secondary(self.path_span.file_id, self.path_span.range())
                    .with_message("in this import"),
            ])
            .with_notes(vec![
                format!(
                    "the module '{}' does not exist or is not visible from this scope",
                    partial_path
                ),
            ])
    }
}

/// Error when trying to import a symbol that doesn't exist in the module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolNotFoundInModuleError {
    /// The symbol name that wasn't found
    pub symbol_name: String,
    /// The module path where we looked
    pub module_path: Vec<String>,
    /// Span of the symbol name
    pub symbol_span: Span,
    /// Span of the module path
    pub module_span: Span,
}

impl IntoDiagnostic for SymbolNotFoundInModuleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let module_name = self.module_path.join(".");

        Diagnostic::error()
            .with_message(format!(
                "symbol '{}' not found in module '{}'",
                self.symbol_name, module_name
            ))
            .with_labels(vec![
                Label::primary(self.symbol_span.file_id, self.symbol_span.range())
                    .with_message(format!("'{}' does not exist", self.symbol_name)),
                Label::secondary(self.module_span.file_id, self.module_span.range())
                    .with_message(format!("in module '{}'", module_name)),
            ])
    }
}

/// Error when trying to import from something that isn't a module
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CannotImportFromNonModuleError {
    /// What we tried to import from
    pub symbol_kind: String,
    /// Path to the symbol
    pub path: Vec<String>,
    /// Span of the path
    pub path_span: Span,
}

impl IntoDiagnostic for CannotImportFromNonModuleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let path_str = self.path.join(".");

        Diagnostic::error()
            .with_message(format!(
                "cannot import from '{}': not a module",
                path_str
            ))
            .with_labels(vec![
                Label::primary(self.path_span.file_id, self.path_span.range())
                    .with_message(format!("this is a {}, not a module", self.symbol_kind)),
            ])
            .with_notes(vec![
                "only modules can be imported from".to_string(),
            ])
    }
}

/// Error when an import creates a name conflict
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportConflictError {
    /// The conflicting name
    pub name: String,
    /// Span of the new import
    pub import_span: Span,
    /// Span of the existing declaration/import
    pub existing_span: Span,
    /// Whether the existing symbol is a declaration or import
    pub existing_is_import: bool,
}

impl IntoDiagnostic for ImportConflictError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let existing_kind = if self.existing_is_import {
            "imported"
        } else {
            "declared"
        };

        Diagnostic::error()
            .with_message(format!("'{}' is already {}", self.name, existing_kind))
            .with_labels(vec![
                Label::primary(self.import_span.file_id, self.import_span.range())
                    .with_message(format!("cannot import '{}'", self.name)),
                Label::secondary(self.existing_span.file_id, self.existing_span.range())
                    .with_message(format!("'{}' first {} here", self.name, existing_kind)),
            ])
    }
}

/// Error when the imported symbol is not visible from the current scope
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolNotVisibleError {
    /// The symbol name
    pub symbol_name: String,
    /// The symbol's visibility level
    pub visibility: String,
    /// Span of the import statement
    pub import_span: Span,
    /// Span of the symbol's declaration (where visibility is declared)
    pub declaration_span: Option<Span>,
}

impl IntoDiagnostic for SymbolNotVisibleError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut labels = vec![
            Label::primary(self.import_span.file_id, self.import_span.range())
                .with_message(format!("'{}' is {}", self.symbol_name, self.visibility)),
        ];

        if let Some(decl_span) = &self.declaration_span {
            labels.push(
                Label::secondary(decl_span.file_id, decl_span.range())
                    .with_message(format!("'{}' declared as {} here", self.symbol_name, self.visibility)),
            );
        }

        Diagnostic::error()
            .with_message(format!("'{}' is not accessible", self.symbol_name))
            .with_labels(labels)
    }
}


/// Error when wrong number of type arguments are provided
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeArityError {
    /// Name of the type being instantiated
    pub type_name: String,
    /// Expected number of required type arguments
    pub expected_min: usize,
    /// Expected maximum (if there are defaults)
    pub expected_max: usize,
    /// Actual number provided
    pub actual: usize,
    /// Span of the type arguments
    pub span: Span,
}

impl IntoDiagnostic for TypeArityError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let expected_msg = if self.expected_min == self.expected_max {
            format!("{}", self.expected_min)
        } else {
            format!("{} to {}", self.expected_min, self.expected_max)
        };

        Diagnostic::error()
            .with_message(format!(
                "wrong number of type arguments for '{}': expected {}, found {}",
                self.type_name, expected_msg, self.actual
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("expected {} type argument(s)", expected_msg)),
            ])
    }
}

/// Error when type arguments are provided to a non-generic type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeNotGenericError {
    /// Name of the type
    pub type_name: String,
    /// Span of the type arguments
    pub span: Span,
}

impl IntoDiagnostic for TypeNotGenericError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type '{}' does not take type arguments",
                self.type_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unexpected type arguments"),
            ])
    }
}

/// Error when duplicate type parameter names are found
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateTypeParameterError {
    /// The duplicated name
    pub name: String,
    /// Span of the duplicate
    pub duplicate_span: Span,
    /// Span of the original
    pub original_span: Span,
}

impl IntoDiagnostic for DuplicateTypeParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("duplicate type parameter '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.duplicate_span.file_id, self.duplicate_span.range())
                    .with_message("duplicate definition"),
                Label::secondary(self.original_span.file_id, self.original_span.range())
                    .with_message("first defined here"),
            ])
    }
}

/// Error when type parameter with default comes before one without
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultOrderingError {
    /// Name of the type parameter with default that is out of order
    pub param_with_default: String,
    /// Name of the type parameter without default that comes after
    pub param_without_default: String,
    /// Span of the parameter with default
    pub with_default_span: Span,
    /// Span of the parameter without default
    pub without_default_span: Span,
}

impl IntoDiagnostic for DefaultOrderingError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "type parameter '{}' with default must come after parameters without defaults",
                self.param_with_default
            ))
            .with_labels(vec![
                Label::primary(self.with_default_span.file_id, self.with_default_span.range())
                    .with_message(format!("'{}' has a default", self.param_with_default)),
                Label::secondary(self.without_default_span.file_id, self.without_default_span.range())
                    .with_message(format!(
                        "'{}' has no default and comes later",
                        self.param_without_default
                    )),
            ])
            .with_notes(vec![
                "type parameters with defaults must be declared after all required parameters"
                    .to_string(),
            ])
    }
}

/// Error when a type in a where clause bound is not a protocol
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonProtocolBoundError {
    /// Name of the type used as a bound
    pub type_name: String,
    /// What kind of type it is (e.g., "struct", "type alias")
    pub type_kind: String,
    /// Span of the bound
    pub span: Span,
}

impl IntoDiagnostic for NonProtocolBoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("'{}' is not a protocol", self.type_name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("'{}' is a {}", self.type_name, self.type_kind)),
            ])
            .with_notes(vec!["only protocols can be used as type bounds".to_string()])
    }
}

/// Error when a type parameter in where clause is not declared
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndeclaredTypeParameterError {
    /// Name of the undeclared type parameter
    pub name: String,
    /// Span of the reference
    pub span: Span,
    /// Available type parameters in scope
    pub available: Vec<String>,
}

impl IntoDiagnostic for UndeclaredTypeParameterError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut notes = vec![];
        if !self.available.is_empty() {
            notes.push(format!(
                "available type parameters: {}",
                self.available.join(", ")
            ));
        }

        Diagnostic::error()
            .with_message(format!(
                "undeclared type parameter '{}' in where clause",
                self.name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("not declared"),
            ])
            .with_notes(notes)
    }
}
