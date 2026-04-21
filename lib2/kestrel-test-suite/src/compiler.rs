//! TestCompiler: high-level wrapper for the lib2 compiler with test assertions.
//!
//! Wraps `Compiler` and provides both a step-by-step API (add_source, infer,
//! analyze, mir, run) and fluent assertion methods (expect_no_errors,
//! expect_error, expect_stdout).

use std::cell::OnceCell;

use kestrel_compiler_driver::{AnalyzeSummary, CompilerDriver, InferSummary};
use kestrel_compiler2::{Compiler, SourceText};
use kestrel_hecs::Entity;
use kestrel_mir::MirModule;

use crate::diagnostic_matcher::{
    self, TestDiagnostic, TestSeverity, from_analyze_diagnostics_with_source,
    from_codespan_diagnostics,
};
use crate::runner::{self, RunResult};

/// A test compiler with lazy phase execution and assertion helpers.
pub struct TestCompiler {
    compiler: Compiler,
    has_stdlib: bool,
    /// Entities for test files (not stdlib files).
    test_files: Vec<(String, Entity)>,
    /// Cached results for lazy evaluation.
    infer_result: OnceCell<InferSummary>,
    analyze_result: OnceCell<AnalyzeSummary>,
}

impl TestCompiler {
    /// Create a test compiler without stdlib.
    pub fn new() -> Self {
        Self {
            compiler: crate::test_compiler(false),
            has_stdlib: false,
            test_files: Vec::new(),
            infer_result: OnceCell::new(),
            analyze_result: OnceCell::new(),
        }
    }

    /// Create a test compiler with stdlib pre-loaded from cache.
    pub fn with_stdlib() -> Self {
        Self {
            compiler: crate::test_compiler(true),
            has_stdlib: true,
            test_files: Vec::new(),
            infer_result: OnceCell::new(),
            analyze_result: OnceCell::new(),
        }
    }

    /// Add a source file, parse, and build declarations.
    ///
    /// Returns the file entity (useful for diagnostic matching).
    pub fn add_source(&mut self, path: &str, source: &str) -> Entity {
        let entity = self.compiler.set_source(path, source.to_string());
        self.compiler.build(entity);
        self.test_files.push((path.to_string(), entity));
        entity
    }

    /// Run type inference on all bodies.
    pub fn infer(&self) -> &InferSummary {
        self.infer_result
            .get_or_init(|| CompilerDriver::new(&self.compiler).infer_all())
    }

    /// Run all registered analyzers.
    pub fn analyze(&self) -> &AnalyzeSummary {
        self.analyze_result
            .get_or_init(|| CompilerDriver::new(&self.compiler).analyze_all())
    }

    /// Collect all diagnostics from all stages as unified TestDiagnostics.
    ///
    /// Runs inference and analysis if not already done.
    pub fn all_diagnostics(&self) -> Vec<TestDiagnostic> {
        // Ensure inference has run (triggers lex/parse/infer diagnostics)
        self.infer();

        let mut result = Vec::new();

        // Collect sources for byte-offset-to-line resolution
        let sources = self.source_map();

        // Codespan diagnostics (lex + parse + infer)
        let codespan_diags = self.compiler.diagnostics();
        result.extend(from_codespan_diagnostics(&codespan_diags, &sources));

        // Analyzer diagnostics. Two dedup passes:
        //  1. Skip E100 — duplicates an inference error already in the codespan stream.
        //  2. Skip any analyzer diag whose (file_id, line, severity, message) is
        //     a prefix of a codespan diag already collected. HIR lowering and the
        //     analyzer both independently emit "cannot find type 'X' in this scope"
        //     for the same unresolved annotation; the codespan version appends a
        //     label suffix (": not found (failed at 'X')"), so prefix-match catches it.
        let analyze_summary = self.analyze();
        let analyzer_diags: Vec<_> = analyze_summary
            .diagnostics
            .iter()
            .filter(|d| d.descriptor_id != "E100")
            .cloned()
            .collect();
        let analyzer_test_diags = from_analyze_diagnostics_with_source(&analyzer_diags, &sources);
        for a in analyzer_test_diags {
            let duplicate = result.iter().any(|c| {
                c.file_id == a.file_id
                    && c.line == a.line
                    && c.severity == a.severity
                    && (c.message == a.message
                        || c.message.starts_with(&format!("{}: ", a.message)))
            });
            if !duplicate {
                result.push(a);
            }
        }

        result
    }

    /// Lower to MIR. Runs inference first if needed.
    pub fn mir(&self) -> MirModule {
        self.infer();
        self.compiler.lower_to_mir()
    }

    /// Compile, link, and run. Returns the run result.
    pub fn run(&self) -> Result<RunResult, String> {
        self.infer();
        runner::compile_and_run(&self.compiler)
    }

    /// Access the underlying compiler.
    pub fn compiler(&self) -> &Compiler {
        &self.compiler
    }

    /// Access the ECS world.
    pub fn world(&self) -> &kestrel_hecs::World {
        self.compiler.world()
    }

    /// Whether stdlib is loaded.
    pub fn has_stdlib(&self) -> bool {
        self.has_stdlib
    }

    /// Get the file entity for the first (or only) test file.
    pub fn test_file_entity(&self) -> Option<Entity> {
        self.test_files.first().map(|(_, e)| *e)
    }

    /// Get all test file entities.
    pub fn test_files(&self) -> &[(String, Entity)] {
        &self.test_files
    }

    // === Source mapping ===

    /// Build a (file_id, source_text) mapping for line resolution.
    fn source_map(&self) -> Vec<(usize, String)> {
        let world = self.compiler.world();
        let mut sources = Vec::new();
        for (_, &entity) in self.compiler.files() {
            if let Some(source) = world.get::<SourceText>(entity) {
                sources.push((entity.index(), source.0.clone()));
            }
        }
        sources
    }

    // === Assertion helpers ===

    /// Check that no errors occurred. Returns Err with details on failure.
    pub fn check_no_errors(&self) -> Result<(), String> {
        let diags = self.all_diagnostics();
        let errors: Vec<&TestDiagnostic> = diags
            .iter()
            .filter(|d| d.severity == TestSeverity::Error)
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            // Build file_id → short path map for readable error output
            let file_names: std::collections::HashMap<usize, String> = self
                .compiler
                .files()
                .iter()
                .map(|(path, entity)| {
                    // Shorten path: just keep the last 2 components (e.g. "iter/iterator.ks")
                    let short = std::path::Path::new(path)
                        .iter()
                        .rev()
                        .take(2)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<std::path::PathBuf>()
                        .to_string_lossy()
                        .to_string();
                    (entity.index(), short)
                })
                .collect();

            let details: Vec<String> = errors
                .iter()
                .map(|d| {
                    let file = file_names
                        .get(&d.file_id)
                        .map(|s| s.as_str())
                        .unwrap_or("?");
                    format!(
                        "  {}:{}: {}{}",
                        file,
                        d.line,
                        d.message,
                        d.code
                            .as_ref()
                            .map(|c| format!(" [{}]", c))
                            .unwrap_or_default()
                    )
                })
                .collect();
            Err(format!(
                "Expected no errors, but found {}:\n{}",
                errors.len(),
                details.join("\n")
            ))
        }
    }

    /// Assert no errors from any stage. Panics with details on failure.
    pub fn expect_no_errors(&self) {
        self.check_no_errors().unwrap_or_else(|e| panic!("{}", e));
    }

    /// Assert that at least one error contains the given message substring.
    pub fn expect_error(&self, message: &str) {
        let diags = self.all_diagnostics();
        let has_match = diags.iter().any(|d| {
            d.severity == TestSeverity::Error
                && d.message.to_lowercase().contains(&message.to_lowercase())
        });
        if !has_match {
            let errors: Vec<String> = diags
                .iter()
                .filter(|d| d.severity == TestSeverity::Error)
                .map(|d| format!("  line {}: {}", d.line, d.message))
                .collect();
            if errors.is_empty() {
                panic!(
                    "Expected error containing '{}', but no errors found",
                    message
                );
            } else {
                panic!(
                    "Expected error containing '{}', but none matched.\nActual errors:\n{}",
                    message,
                    errors.join("\n")
                );
            }
        }
    }

    /// Assert that at least one error has the given analyzer code.
    pub fn expect_error_code(&self, code: &str) {
        let diags = self.all_diagnostics();
        let has_match = diags
            .iter()
            .any(|d| d.severity == TestSeverity::Error && d.code.as_deref() == Some(code));
        if !has_match {
            panic!("Expected error with code '{}', but none found", code);
        }
    }

    /// Assert the MIR output contains the given string.
    pub fn expect_mir_contains(&self, needle: &str) {
        let mir = self.mir();
        let mir_text = format!("{}", mir.display());
        if !mir_text.contains(needle) {
            panic!(
                "Expected MIR to contain '{}'\n\nActual MIR:\n{}",
                needle, mir_text
            );
        }
    }

    /// Assert the program runs with exit code 0.
    pub fn expect_runs(&self) {
        match self.run() {
            Ok(result) => {
                if result.exit_code != 0 {
                    panic!(
                        "Expected exit code 0, got {}\nstdout: {}\nstderr: {}",
                        result.exit_code, result.stdout, result.stderr
                    );
                }
            },
            Err(e) => panic!("Compilation/execution failed: {}", e),
        }
    }

    /// Assert the program's stdout matches exactly (trimmed).
    pub fn expect_stdout(&self, expected: &str) {
        match self.run() {
            Ok(result) => {
                if result.stdout.trim() != expected.trim() {
                    panic!(
                        "Stdout mismatch.\nExpected: {}\nActual:   {}\nstderr: {}",
                        expected.trim(),
                        result.stdout.trim(),
                        result.stderr
                    );
                }
            },
            Err(e) => panic!("Compilation/execution failed: {}", e),
        }
    }

    /// Assert the program's stdout contains a substring.
    pub fn expect_stdout_contains(&self, needle: &str) {
        match self.run() {
            Ok(result) => {
                if !result.stdout.contains(needle) {
                    panic!(
                        "Expected stdout to contain '{}'\nActual stdout: {}\nstderr: {}",
                        needle, result.stdout, result.stderr
                    );
                }
            },
            Err(e) => panic!("Compilation/execution failed: {}", e),
        }
    }

    /// Assert the program exits with a specific code.
    pub fn expect_exit_code(&self, code: i32) {
        match self.run() {
            Ok(result) => {
                if result.exit_code != code {
                    panic!(
                        "Expected exit code {}, got {}\nstdout: {}\nstderr: {}",
                        code, result.exit_code, result.stdout, result.stderr
                    );
                }
            },
            Err(e) => panic!("Compilation/execution failed: {}", e),
        }
    }

    /// Check inline annotations against diagnostics (two-way match).
    pub fn check_annotations(
        &self,
        annotations: &[crate::annotation::Annotation],
        test_file_id: usize,
    ) -> Result<(), String> {
        let diags = self.all_diagnostics();
        if std::env::var("KTS_DUMP_DIAGS").is_ok() {
            eprintln!("== DIAGS (file_id={}) ==", test_file_id);
            for d in &diags {
                eprintln!(
                    "  file_id={} line={} sev={:?} msg={:?}",
                    d.file_id, d.line, d.severity, d.message
                );
            }
        }
        diagnostic_matcher::check(annotations, &diags, test_file_id)
    }
}

impl Default for TestCompiler {
    fn default() -> Self {
        Self::new()
    }
}
