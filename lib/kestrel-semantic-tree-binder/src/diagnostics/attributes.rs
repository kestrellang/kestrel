//! Attribute-related warnings.
//!
//! Warnings emitted for unknown or invalid attributes.

use kestrel_reporting::{Diagnostic, IntoDiagnostic, Label};
use kestrel_span::Span;

/// Warning when an unknown attribute is used.
pub struct UnknownAttributeWarning {
    /// The name of the unknown attribute
    pub name: String,
    /// The span of the attribute
    pub span: Span,
}

impl IntoDiagnostic for UnknownAttributeWarning {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!("unknown attribute '{}'", self.name))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unknown attribute"),
            ])
    }
}

// ============================================================================
// File Constant Attribute Errors
// ============================================================================

/// Error when @fileconstant is missing a path argument.
pub struct FileConstantRequiresPathError {
    pub span: Span,
}

impl IntoDiagnostic for FileConstantRequiresPathError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@fileconstant requires a file path argument")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("expected @fileconstant(\"path/to/file.bin\")"),
            ])
    }
}

/// Error when @fileconstant has an invalid argument format.
pub struct FileConstantInvalidArgumentError {
    pub span: Span,
}

impl IntoDiagnostic for FileConstantInvalidArgumentError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@fileconstant argument must be an unlabeled string")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("expected a string literal path"),
            ])
    }
}

/// Error when @fileconstant argument is not a string literal.
pub struct FileConstantRequiresStringError {
    pub span: Span,
}

impl IntoDiagnostic for FileConstantRequiresStringError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@fileconstant requires a string literal path")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("expected a string literal like \"file.bin\""),
            ])
    }
}

/// Error when the file specified in @fileconstant cannot be found.
pub struct FileConstantFileNotFoundError {
    pub span: Span,
    pub path: String,
}

impl IntoDiagnostic for FileConstantFileNotFoundError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("file not found: {}", self.path))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("file does not exist"),
            ])
    }
}

/// Error when the file specified in @fileconstant cannot be read.
pub struct FileConstantReadError {
    pub span: Span,
    pub path: String,
    pub error: String,
}

impl IntoDiagnostic for FileConstantReadError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!("failed to read file: {}", self.path))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message(format!("read error: {}", self.error)),
            ])
    }
}

/// Error when the file size is not aligned to element size.
pub struct FileConstantInvalidSizeError {
    pub span: Span,
    pub file_size: usize,
    pub element_size: usize,
}

impl IntoDiagnostic for FileConstantInvalidSizeError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("file size is not aligned to element size")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range()).with_message(format!(
                    "file is {} bytes, but element size is {} bytes (remainder: {})",
                    self.file_size,
                    self.element_size,
                    self.file_size % self.element_size
                )),
            ])
    }
}

/// Error when @fileconstant is used on a non-LiteralSlice type.
pub struct FileConstantRequiresLiteralSliceError {
    pub span: Span,
}

impl IntoDiagnostic for FileConstantRequiresLiteralSliceError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("@fileconstant requires LiteralSlice[T] type")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("type must be LiteralSlice[T]"),
            ])
    }
}
