use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use std::collections::HashMap;

// Re-export commonly used types from codespan_reporting
pub use codespan_reporting::diagnostic::{Diagnostic, Label, Severity};
pub use codespan_reporting::files;

/// Emit diagnostics to stderr using any `Files` implementation.
///
/// Unlike `DiagnosticContext::emit()` which uses its own `SimpleFiles`,
/// this accepts any `Files` impl — useful when file storage lives
/// elsewhere (e.g. an ECS world).
pub fn emit_all<'a, F>(
    files: &'a F,
    diagnostics: &[Diagnostic<usize>],
) -> Result<(), codespan_reporting::files::Error>
where
    F: codespan_reporting::files::Files<'a, FileId = usize>,
{
    let writer = StandardStream::stderr(ColorChoice::Always);
    let config = term::Config::default();
    for diagnostic in diagnostics {
        term::emit_to_write_style(&mut writer.lock(), &config, files, diagnostic)?;
    }
    Ok(())
}

/// Trait for types that can be converted into a diagnostic.
/// Implement this for your error types to integrate with the reporting system.
///
/// The file ID is extracted from the span(s) stored in the error type.
pub trait ToDiagnostic {
    fn to_diagnostic(&self) -> Diagnostic<usize>;
}

/// Context for managing and reporting diagnostics.
/// Collects diagnostics and source files, then emits them to the terminal.
pub struct DiagnosticContext {
    files: SimpleFiles<String, String>,
    diagnostics: Vec<Diagnostic<usize>>,
    file_map: HashMap<String, usize>,
}

impl DiagnosticContext {
    pub fn new() -> Self {
        Self {
            files: SimpleFiles::new(),
            diagnostics: Vec::new(),
            file_map: HashMap::new(),
        }
    }

    /// Register a source file. Returns the file ID. Deduplicates by name.
    pub fn add_file(&mut self, name: String, source: String) -> usize {
        if let Some(&id) = self.file_map.get(&name) {
            return id;
        }
        let id = self.files.add(name.clone(), source);
        self.file_map.insert(name, id);
        id
    }

    /// Convert and add a diagnostic via the ToDiagnostic trait.
    pub fn throw<D: ToDiagnostic>(&mut self, diagnostic: D) {
        self.diagnostics.push(diagnostic.to_diagnostic());
    }

    /// Add a raw pre-built diagnostic.
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic<usize>) {
        self.diagnostics.push(diagnostic);
    }

    /// True if any error or bug diagnostics have been collected.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error || d.severity == Severity::Bug)
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Emit all diagnostics to stderr with color support.
    pub fn emit(&self) -> Result<(), codespan_reporting::files::Error> {
        let writer = StandardStream::stderr(ColorChoice::Always);
        self.emit_diagnostics(&mut writer.lock(), &self.diagnostics)
    }

    /// Emit additional diagnostics (e.g. from lowering/codegen) that weren't
    /// collected during the original compilation phase.
    pub fn emit_additional(
        &self,
        diagnostics: &[Diagnostic<usize>],
    ) -> Result<(), codespan_reporting::files::Error> {
        let writer = StandardStream::stderr(ColorChoice::Always);
        self.emit_diagnostics(&mut writer.lock(), diagnostics)
    }

    /// Emit all diagnostics to a custom writer.
    pub fn emit_to<W: term::termcolor::WriteColor>(
        &self,
        writer: &mut W,
    ) -> Result<(), codespan_reporting::files::Error> {
        self.emit_diagnostics(writer, &self.diagnostics)
    }

    /// Clear all diagnostics (keeps registered files).
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }

    pub fn diagnostics(&self) -> &[Diagnostic<usize>] {
        &self.diagnostics
    }

    /// Look up a file ID by name.
    pub fn get_file_id(&self, name: &str) -> Option<usize> {
        self.file_map.get(name).copied()
    }

    fn emit_diagnostics<W: term::termcolor::WriteColor>(
        &self,
        writer: &mut W,
        diagnostics: &[Diagnostic<usize>],
    ) -> Result<(), codespan_reporting::files::Error> {
        let config = codespan_reporting::term::Config::default();
        for diagnostic in diagnostics {
            term::emit_to_write_style(writer, &config, &self.files, diagnostic)?;
        }
        Ok(())
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
