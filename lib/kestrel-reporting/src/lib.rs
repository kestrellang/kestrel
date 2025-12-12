use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::collections::HashMap;

// Re-export commonly used types from codespan_reporting
pub use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};

/// Trait for types that can be converted into a diagnostic.
/// Implement this for your error types to integrate with the reporting system.
///
/// The file ID is extracted from the span(s) stored in the error type.
#[allow(clippy::wrong_self_convention)]
pub trait IntoDiagnostic {
    /// Convert this error into a codespan diagnostic.
    fn into_diagnostic(&self) -> Diagnostic<usize>;
}

/// Context for managing and reporting diagnostics.
/// This struct collects diagnostics and can emit them to the terminal.
pub struct DiagnosticContext {
    files: SimpleFiles<String, String>,
    diagnostics: Vec<Diagnostic<usize>>,
    file_map: HashMap<String, usize>,
}

impl DiagnosticContext {
    /// Create a new diagnostic context.
    pub fn new() -> Self {
        Self {
            files: SimpleFiles::new(),
            diagnostics: Vec::new(),
            file_map: HashMap::new(),
        }
    }

    /// Add a source file to the context.
    /// Returns the file ID that can be used when creating diagnostics.
    pub fn add_file(&mut self, name: String, source: String) -> usize {
        if let Some(&id) = self.file_map.get(&name) {
            return id;
        }
        let id = self.files.add(name.clone(), source);
        self.file_map.insert(name, id);
        id
    }

    /// Throw (add) a diagnostic to the context.
    pub fn throw<D: IntoDiagnostic>(&mut self, diagnostic: D) {
        self.diagnostics.push(diagnostic.into_diagnostic());
    }

    /// Add a raw diagnostic to the context.
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic<usize>) {
        self.diagnostics.push(diagnostic);
    }

    /// Check if there are any errors in the collected diagnostics.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error || d.severity == Severity::Bug)
    }

    /// Get the number of diagnostics collected.
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Check if the context is empty (no diagnostics).
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Emit all diagnostics to stderr with color support.
    pub fn emit(&self) -> Result<(), codespan_reporting::files::Error> {
        let writer = StandardStream::stderr(ColorChoice::Auto);
        let config = codespan_reporting::term::Config::default();

        for diagnostic in &self.diagnostics {
            term::emit_to_io_write(&mut writer.lock(), &config, &self.files, diagnostic)?;
        }

        Ok(())
    }

    /// Emit all diagnostics to a custom writer.
    pub fn emit_to<W: term::termcolor::WriteColor>(
        &self,
        writer: &mut W,
    ) -> Result<(), codespan_reporting::files::Error> {
        let config = codespan_reporting::term::Config::default();

        for diagnostic in &self.diagnostics {
            term::emit_to_io_write(writer, &config, &self.files, diagnostic)?;
        }

        Ok(())
    }

    /// Clear all diagnostics (keeps the files).
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    /// Get a reference to all diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic<usize>] {
        &self.diagnostics
    }

    /// Get a file ID by name, if it exists.
    pub fn get_file_id(&self, name: &str) -> Option<usize> {
        self.file_map.get(name).copied()
    }
}

impl Default for DiagnosticContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro to create a simple diagnostic.
#[macro_export]
macro_rules! diagnostic {
    (error, $($args:tt)*) => {
        $crate::Diagnostic::error().with_message(format!($($args)*))
    };
    (warning, $($args:tt)*) => {
        $crate::Diagnostic::warning().with_message(format!($($args)*))
    };
    (note, $($args:tt)*) => {
        $crate::Diagnostic::note().with_message(format!($($args)*))
    };
}
