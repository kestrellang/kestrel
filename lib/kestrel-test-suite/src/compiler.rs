//! TestCompiler: high-level wrapper for the lib compiler with test assertions.
//!
//! Wraps `Compiler` and provides both a step-by-step API (add_source, infer,
//! analyze, mir, run) and fluent assertion methods (expect_no_errors,
//! expect_error, expect_stdout).

use std::cell::OnceCell;

use kestrel_compiler::{Compiler, SourceText};
use kestrel_compiler_driver::{AnalyzeSummary, CompilerDriver, InferSummary};
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
    /// Whether analysis treats this as an executable build (gates the
    /// entry-point requirement E618). Execution tests set this true; diagnostics
    /// tests default false and opt in via `// executable: true`.
    is_executable: bool,
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
            is_executable: false,
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
            is_executable: false,
            test_files: Vec::new(),
            infer_result: OnceCell::new(),
            analyze_result: OnceCell::new(),
        }
    }

    /// Mark this compilation as producing an executable, so analysis enforces
    /// the entry-point requirement (E618). Must be called before `analyze()`.
    pub fn set_executable(&mut self, is_executable: bool) {
        self.is_executable = is_executable;
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
            .get_or_init(|| CompilerDriver::new(&self.compiler).analyze_all(self.is_executable))
    }

    /// Collect all diagnostics from all stages as unified TestDiagnostics:
    /// codespan (lex / parse / infer) and HIR-level analyzers.
    pub fn all_diagnostics(&self) -> Vec<TestDiagnostic> {
        // Ensure inference has run (triggers lex/parse/infer diagnostics)
        self.infer();

        // Run analyzers BEFORE snapshotting codespan diagnostics: analyzers
        // may force lowering queries (e.g. the stage-0.5 ref rejection on a
        // field no body touches) whose accumulated codespan diagnostics
        // would otherwise be dropped.
        let analyze_summary = self.analyze();

        let mut result = Vec::new();

        // Collect sources for byte-offset-to-line resolution
        let sources = self.source_map();

        // Codespan diagnostics (lex + parse + infer + analyzer-forced lowering)
        let codespan_diags = self.compiler.diagnostics();
        result.extend(from_codespan_diagnostics(&codespan_diags, &sources));

        // Analyzer diagnostics. Two dedup passes:
        //  1. Skip E100 — duplicates an inference error already in the codespan stream.
        //  2. Skip any analyzer diag whose (file_id, line, severity, message) is
        //     a prefix of a codespan diag already collected. HIR lowering and the
        //     analyzer both independently emit "cannot find type 'X' in this scope"
        //     for the same unresolved annotation; the codespan version appends a
        //     label suffix (": not found (failed at 'X')"), so prefix-match catches it.
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

        // MIR-stage diagnostics (escape check E494-E496, ref-across-merge
        // E497): the MIR pipeline never ran for diagnostics tests before, so
        // these were invisible. Run lowering only when the front end is clean
        // (error-recovery HIR would ICE-spam) and append only CODED
        // diagnostics — uncoded verify errors are ICEs, not an annotatable
        // surface (and lower_to_mir suppresses them as cascade noise anyway).
        let front_end_clean = !result.iter().any(|d| d.severity == TestSeverity::Error);
        if front_end_clean {
            let before = self.compiler.diagnostics();
            let _ = self.compiler.lower_to_mir();
            let new: Vec<_> = self
                .compiler
                .diagnostics()
                .into_iter()
                .filter(|d| !before.contains(d))
                .collect();
            result.extend(
                from_codespan_diagnostics(&new, &sources)
                    .into_iter()
                    .filter(|d| d.code.is_some()),
            );
        }

        result
    }

    /// Lower to MIR (OSSA). Runs inference first if needed.
    pub fn mir(&self) -> Result<MirModule, String> {
        self.infer();
        self.compiler
            .lower_to_mir()
            .map_err(|e| format!("MIR lowering failed: {e}"))
    }

    /// Compile, link, and run on the env-default backend.
    pub fn run(&self) -> Result<RunResult, String> {
        self.run_on(runner::Backend::default_from_env())
    }

    /// Compile, link, and run on a specific backend (`// backends:` trials).
    pub fn run_on(&self, backend: runner::Backend) -> Result<RunResult, String> {
        self.infer();
        runner::compile_and_run(&self.compiler, backend)
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
        for &entity in self.compiler.files().values() {
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
        let mir = match self.mir() {
            Ok(m) => m,
            Err(e) => panic!("{e}"),
        };
        let mir_text = kestrel_mir::display::display_module(&mir);
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
