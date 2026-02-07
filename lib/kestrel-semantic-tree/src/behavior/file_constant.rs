//! File constant behavior for embedding binary file data.
//!
//! This behavior is attached to static fields declared with `@fileconstant("path")`
//! to indicate they should embed binary file data directly into the executable.

use kestrel_span::Span;
use semantic_tree::behavior::Behavior;

use crate::behavior::KestrelBehaviorKind;
use crate::language::KestrelLanguage;

/// Behavior for fields declared with `@fileconstant("path.bin")`.
///
/// This behavior indicates that the field:
/// - Has its data read from a file at compile time
/// - The data is embedded directly in .rodata
/// - The type must be `LiteralSlice[T]`
/// - No runtime initialization is needed
///
/// # Example
///
/// ```kestrel
/// @fileconstant("unicode_tables.bin")
/// let UPPER_CASE_TABLE: LiteralSlice[Int32]
/// ```
#[derive(Debug, Clone)]
pub struct FileConstantBehavior {
    /// The relative file path as specified in the attribute
    relative_path: String,
    /// The span of the attribute for error reporting
    span: Span,
    /// The embedded file data (populated after file is read)
    data: Option<Vec<u8>>,
}

impl FileConstantBehavior {
    /// Create a new FileConstantBehavior with just the path.
    /// Data will be populated later during lowering.
    pub fn new(relative_path: String, span: Span) -> Self {
        Self {
            relative_path,
            span,
            data: None,
        }
    }

    /// Create a FileConstantBehavior with data already loaded.
    pub fn with_data(relative_path: String, span: Span, data: Vec<u8>) -> Self {
        Self {
            relative_path,
            span,
            data: Some(data),
        }
    }

    /// Get the relative file path.
    pub fn relative_path(&self) -> &str {
        &self.relative_path
    }

    /// Get the span of the attribute.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Get the embedded data, if loaded.
    pub fn data(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    /// Set the embedded data.
    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = Some(data);
    }

    /// Check if data has been loaded.
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }
}

impl Behavior<KestrelLanguage> for FileConstantBehavior {
    fn kind(&self) -> KestrelBehaviorKind {
        KestrelBehaviorKind::FileConstant
    }
}
