//! Code generation integration tests.
//!
//! These tests compile Kestrel source code to native executables
//! and verify the output.
//!
//! By default, `compile_and_run` includes a prelude module with builtin protocols
//! (`Copyable`, `Cloneable`). Use `compile_and_run_without_prelude` to opt out.

use std::path::PathBuf;
use std::process::Command;

use kestrel_test_suite::PRELUDE_SOURCE;

// Re-export test modules
mod arithmetic;
mod casts;
mod closures;
mod control_flow;
mod enums;
mod functions;
mod generics;
mod loops;
mod pointers;
mod raii;
mod strings;
mod structs;
mod tuples;

/// Result of running a compiled program.
#[derive(Debug)]
pub struct RunResult {
    /// Exit code of the program.
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

/// Compile Kestrel source code and run the resulting executable.
///
/// Includes the prelude module with builtin protocols by default.
/// Tests can import these with `import Prelude` or `import Prelude.(Copyable, Cloneable)`.
pub fn compile_and_run(source: &str) -> RunResult {
    compile_and_run_impl(source, true)
}

/// Compile Kestrel source code without the prelude module.
///
/// Use this for tests that define their own builtin protocols.
#[allow(dead_code)]
pub fn compile_and_run_without_prelude(source: &str) -> RunResult {
    compile_and_run_impl(source, false)
}

fn compile_and_run_impl(source: &str, include_prelude: bool) -> RunResult {
    // Create unique temp directory for this test (use thread id + timestamp)
    let temp_dir = std::env::temp_dir().join(format!(
        "kestrel_test_{}_{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    // Compile using the full pipeline
    let result = compile_source(source, &temp_dir, include_prelude);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);

    result
}

/// Internal compilation function.
fn compile_source(source: &str, temp_dir: &PathBuf, include_prelude: bool) -> RunResult {
    use kestrel_lexer::lex;
    use kestrel_parser::{Parser, parse_source_file};
    use kestrel_reporting::DiagnosticContext;
    use kestrel_semantic_tree_binder::SemanticBinder;
    use kestrel_semantic_tree_builder::SemanticModelBuilder;
    use kestrel_span::Span;

    let mut builder = SemanticModelBuilder::new();
    let mut diagnostics = DiagnosticContext::new();

    // Collect all files to compile (prelude first if enabled, then test file)
    let mut all_files: Vec<(&str, &str)> = Vec::new();
    if include_prelude {
        all_files.push((PRELUDE_SOURCE.0, PRELUDE_SOURCE.1));
    }
    all_files.push(("test.kes", source));

    // Parse and add all files
    for (file_name, content) in all_files {
        let file_id = diagnostics.add_file(file_name.to_string(), content.to_string());
        let tokens: Vec<_> = lex(content, file_id)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let result = Parser::parse(content, tokens.into_iter(), parse_source_file, file_id);

        if !result.errors.is_empty() {
            // Add parse errors to diagnostics
            for error in &result.errors {
                let span = error.span.clone().unwrap_or(Span::from(0..1));
                let diagnostic = kestrel_reporting::Diagnostic::error()
                    .with_message(&error.message)
                    .with_labels(vec![kestrel_reporting::Label::primary(
                        file_id,
                        span.range(),
                    )]);
                diagnostics.add_diagnostic(diagnostic);
            }
        }

        builder.add_file(file_name, &result.tree, content, file_id, &mut diagnostics);
    }

    // Check for parse errors
    if diagnostics.has_errors() {
        let errors: Vec<_> = diagnostics
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect();
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Parse errors: {}", errors.join("\n")),
        };
    }
    let model = builder.build();
    let model = SemanticBinder::bind(model, &mut diagnostics);

    // Run analyzers
    {
        use kestrel_semantic_analyzers::{AnalysisContext, Analyzer, default_analyzers, run_all};
        let mut owned = default_analyzers();
        let mut analyzers: Vec<&mut dyn Analyzer> = Vec::new();
        for a in owned.iter_mut() {
            analyzers.push(a.as_mut());
        }
        let mut ctx = AnalysisContext::new(&model, &mut diagnostics);
        run_all(&mut analyzers, &model, &mut ctx);
    }

    if diagnostics.has_errors() {
        let errors: Vec<_> = diagnostics
            .diagnostics()
            .iter()
            .map(|d| d.message.clone())
            .collect();
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Semantic errors: {}", errors.join("\n")),
        };
    }

    // Lower to MIR
    let root = model.root();
    let mut lowering_result = kestrel_execution_graph_lowering::lower_module(&model, &root);

    if !lowering_result.diagnostics.is_empty() {
        let errors: Vec<_> = lowering_result
            .diagnostics
            .iter()
            .map(|d| format!("{:?}", d))
            .collect();
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Lowering errors: {}", errors.join("\n")),
        };
    }

    // Codegen
    use kestrel_codegen::TargetConfig;
    use kestrel_codegen_cranelift::{CodegenOptions, compile_and_link};

    let target = TargetConfig::host();
    let options = CodegenOptions::default();
    let exe_path = temp_dir.join("test");

    if let Err(e) = compile_and_link(&mut lowering_result.mir, &target, &options, &exe_path) {
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Codegen error: {}", e),
        };
    }

    // Run the executable
    run_executable(&exe_path)
}

/// Run an executable and capture its output.
fn run_executable(path: &PathBuf) -> RunResult {
    match Command::new(path).output() {
        Ok(output) => RunResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(e) => RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Failed to run executable: {}", e),
        },
    }
}
