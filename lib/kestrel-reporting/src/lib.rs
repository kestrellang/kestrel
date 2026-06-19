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
    let mut last_err = Ok(());
    for diagnostic in diagnostics {
        // Render each diagnostic independently: one with an unresolvable
        // (synthetic/missing) span must not abort the whole batch.
        if let Err(e) = emit_one(&mut writer.lock(), &config, files, diagnostic) {
            last_err = Err(e);
        }
    }
    last_err
}

/// Emit a single diagnostic; never let an unresolvable span silence it.
///
/// A diagnostic attached to a synthesized (spanless) node carries a synthetic
/// span pointing at a fileless entity. codespan's file lookup then returns a
/// `files::Error` (e.g. `FileMissing`), which — if propagated — aborts the
/// entire emit batch and the message is lost (a silent build failure). On any
/// such span-derived error we re-emit the diagnostic with its labels stripped,
/// so the message text always reaches the writer even without source context.
/// Only a genuine write/IO failure on the stripped retry is surfaced.
fn emit_one<'a, W, F>(
    writer: &mut W,
    config: &term::Config,
    files: &'a F,
    diagnostic: &Diagnostic<usize>,
) -> Result<(), codespan_reporting::files::Error>
where
    W: term::termcolor::WriteColor,
    F: codespan_reporting::files::Files<'a, FileId = usize>,
{
    match term::emit_to_write_style(writer, config, files, diagnostic) {
        Ok(()) => Ok(()),
        // `Io` is a real writer failure stripping labels won't fix; surface it.
        Err(e @ files::Error::Io(_)) => Err(e),
        // Any span/file-lookup error: retry without labels (which is the only
        // part needing source context) so the message still prints.
        Err(_) => term::emit_to_write_style(writer, config, files, &strip_labels(diagnostic)),
    }
}

/// Clone a diagnostic without its labels, appending an explanatory note so the
/// reader knows the location was unavailable rather than simply omitted.
fn strip_labels(diagnostic: &Diagnostic<usize>) -> Diagnostic<usize> {
    let mut bare = diagnostic.clone();
    bare.labels.clear();
    bare.notes.push(
        "(no source location available — diagnostic attached to a synthesized node)".into(),
    );
    bare
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
        let mut last_err = Ok(());
        for diagnostic in diagnostics {
            // Resilient per-diagnostic emit: a synthetic-span diagnostic must
            // not abort the batch (see `emit_one`).
            if let Err(e) = emit_one(writer, &config, &self.files, diagnostic) {
                last_err = Err(e);
            }
        }
        last_err
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

#[cfg(test)]
mod tests {
    use super::*;
    use codespan_reporting::files::SimpleFiles;
    use codespan_reporting::term::termcolor::Buffer;

    fn render(files: &SimpleFiles<String, String>, diags: &[Diagnostic<usize>]) -> String {
        let mut buf = Buffer::no_color();
        let config = term::Config::default();
        let mut last = Ok(());
        for d in diags {
            if let Err(e) = emit_one(&mut buf, &config, files, d) {
                last = Err(e);
            }
        }
        last.expect("emit_one should not fail on synthetic spans");
        String::from_utf8(buf.into_inner()).unwrap()
    }

    /// A diagnostic whose label points at a file absent from the registry
    /// (the synthetic-span case) must still print its message, not vanish.
    #[test]
    fn synthetic_span_diagnostic_still_prints() {
        let files = SimpleFiles::<String, String>::new(); // empty: file id 0 missing
        let diag = Diagnostic::error()
            .with_message("conformance failed on a synthesized node")
            .with_labels(vec![Label::primary(0usize, 0..0)]);
        let out = render(&files, &[diag]);
        assert!(out.contains("conformance failed on a synthesized node"), "got: {out:?}");
        assert!(out.contains("no source location available"), "got: {out:?}");
    }

    /// A bad-span diagnostic earlier in the batch must not suppress later
    /// diagnostics that have valid spans.
    #[test]
    fn bad_span_does_not_abort_batch() {
        let mut files = SimpleFiles::<String, String>::new();
        let fid = files.add("real.ks".to_string(), "let x = 1;\n".to_string());
        let bad = Diagnostic::error()
            .with_message("first: synthetic")
            .with_labels(vec![Label::primary(999usize, 0..0)]);
        let good = Diagnostic::error()
            .with_message("second: real span")
            .with_labels(vec![Label::primary(fid, 4..5)]);
        let out = render(&files, &[bad, good]);
        assert!(out.contains("first: synthetic"), "got: {out:?}");
        assert!(out.contains("second: real span"), "got: {out:?}");
        assert!(out.contains("real.ks"), "real-span diagnostic should show source: {out:?}");
    }
}
