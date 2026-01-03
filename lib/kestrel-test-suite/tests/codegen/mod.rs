//! Code generation integration tests.
//!
//! These tests compile Kestrel source code to native executables
//! and verify the output.

use std::path::PathBuf;
use std::process::Command;

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
/// Returns the exit code, stdout, and stderr.
pub fn compile_and_run(source: &str) -> RunResult {
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
    let result = compile_source(source, &temp_dir);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);

    result
}

/// Internal compilation function.
fn compile_source(source: &str, temp_dir: &PathBuf) -> RunResult {
    use kestrel_lexer::lex;
    use kestrel_parser::{parse_source_file, Parser};
    use kestrel_reporting::DiagnosticContext;
    use kestrel_semantic_tree_binder::SemanticBinder;
    use kestrel_semantic_tree_builder::SemanticModelBuilder;

    let mut builder = SemanticModelBuilder::new();
    let mut diagnostics = DiagnosticContext::new();

    let file_id = diagnostics.add_file("test.kes".to_string(), source.to_string());
    let tokens: Vec<_> = lex(source, file_id)
        .filter_map(|t| t.ok())
        .map(|spanned| (spanned.value, spanned.span))
        .collect();

    let result = Parser::parse(source, tokens.into_iter(), parse_source_file);

    if !result.errors.is_empty() {
        let errors: Vec<_> = result.errors.iter().map(|e| e.message.clone()).collect();
        return RunResult {
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("Parse errors: {}", errors.join("\n")),
        };
    }

    builder.add_file("test.kes", &result.tree, source, file_id, &mut diagnostics);
    let model = builder.build();
    let model = SemanticBinder::bind(model, &mut diagnostics);

    // Run analyzers
    {
        use kestrel_semantic_analyzers::{default_analyzers, run_all, AnalysisContext, Analyzer};
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
    use kestrel_codegen_cranelift::{compile_and_link, CodegenOptions};

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
