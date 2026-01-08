use crate::error::CompileError;
use crate::run::RunResult;
use crate::source_file::SourceFile;
use kestrel_codegen::TargetConfig;
use kestrel_codegen_cranelift::CodegenOptions;
use kestrel_execution_graph_lowering::LoweringResult;
use kestrel_lexer::lex;
use kestrel_parser::{Parser, parse_source_file};
use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_analyzers::{AnalysisContext, Analyzer, run_all};
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree_binder::SemanticBinder;
use kestrel_semantic_tree_builder::SemanticModelBuilder;
use kestrel_span::Span;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// Represents a compiled Kestrel project.
///
/// Contains all compiled source files, a semantic model, and collected diagnostics.
/// Created via `Compilation::builder()`.
pub struct Compilation {
    source_files: Vec<SourceFile>,
    semantic_model: Option<SemanticModel>,
    diagnostics: DiagnosticContext,
}

impl Compilation {
    /// Create a new compilation builder.
    ///
    /// # Example
    /// ```no_run
    /// # use kestrel_compiler::Compilation;
    /// let compilation = Compilation::builder()
    ///     .add_source("main.ks", "module Main\nclass Foo {}")
    ///     .build();
    /// ```
    pub fn builder() -> crate::CompilationBuilder {
        crate::CompilationBuilder::new()
    }

    /// Internal method to create a compilation from source files.
    pub(crate) fn from_sources(sources: Vec<(String, String)>) -> Self {
        let mut diagnostics = DiagnosticContext::new();
        let mut source_files = Vec::new();

        // Create the semantic model builder (build/lowering phase)
        let mut builder = SemanticModelBuilder::new();

        // Phase 1, 2 & 3: Lex, parse, and add each file to the semantic tree
        for (name, source) in sources {
            // Add source file to diagnostics context
            let file_id = diagnostics.add_file(name.clone(), source.clone());

            // Phase 1: Lexing
            let lex_results: Vec<_> = lex(&source, file_id).collect();

            // Collect lex errors
            for result in &lex_results {
                if let Err(error) = result {
                    let error_diag = LexError {
                        span: error.span.clone(),
                    };
                    diagnostics.throw(error_diag);
                }
            }

            // Extract tokens for parsing
            let tokens: Vec<_> = lex_results
                .into_iter()
                .filter_map(|r| r.ok())
                .map(|spanned| (spanned.value, spanned.span))
                .collect();

            // Phase 2: Parsing
            let parse_result =
                Parser::parse(&source, tokens.into_iter(), parse_source_file, file_id);

            // Collect parse errors
            for error in &parse_result.errors {
                let error_diag = ParseErrorDiagnostic {
                    message: error.message.clone(),
                    span: error.span.clone(),
                };
                diagnostics.throw(error_diag);
            }

            // Phase 3: Add file to the semantic tree builder
            builder.add_file(
                &name,
                &parse_result.tree,
                &source,
                file_id,
                &mut diagnostics,
            );

            // Create source file
            let source_file = SourceFile::new(name, source, parse_result.tree);

            source_files.push(source_file);
        }

        // Build the semantic model (lowering)
        let model = builder.build();

        // Run binding phase on the built model
        let model = SemanticBinder::bind(model, &mut diagnostics);

        // Run extracted analyzers after binding (keeps output consistent during migration)
        {
            // Build default analyzers; expand as more validators migrate
            let mut owned = kestrel_semantic_analyzers::default_analyzers();
            let mut analyzers: Vec<&mut dyn Analyzer> = Vec::new();
            for a in owned.iter_mut() {
                analyzers.push(a.as_mut());
            }
            let mut ctx = AnalysisContext::new(&model, &mut diagnostics);
            run_all(&mut analyzers, &model, &mut ctx);
        }

        Self {
            source_files,
            semantic_model: Some(model),
            diagnostics,
        }
    }

    /// Get all compiled source files.
    pub fn source_files(&self) -> &[SourceFile] {
        &self.source_files
    }

    /// Get the semantic model for the entire compilation.
    ///
    /// This contains symbols from all source files in the compilation.
    /// Returns `None` if the compilation has no source files.
    pub fn semantic_model(&self) -> Option<&SemanticModel> {
        self.semantic_model.as_ref()
    }

    /// Get the diagnostic context.
    pub fn diagnostics(&self) -> &DiagnosticContext {
        &self.diagnostics
    }

    /// Check if there are any errors in the compilation.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Get a specific source file by name.
    pub fn get_source_file(&self, name: &str) -> Option<&SourceFile> {
        self.source_files.iter().find(|f| f.name() == name)
    }

    /// Lower the compilation to an execution graph (MIR).
    ///
    /// Returns an error if there are semantic errors or no semantic model.
    pub fn lower_to_execution_graph(&self) -> Result<LoweringResult, CompileError> {
        if self.has_errors() {
            return Err(CompileError::SemanticErrors);
        }

        let model = self
            .semantic_model
            .as_ref()
            .ok_or(CompileError::NoSemanticModel)?;

        let root = model.root();
        let result = kestrel_execution_graph_lowering::lower_module(model, &root);

        if result.has_errors() {
            return Err(CompileError::LoweringFailed(result.diagnostics));
        }

        Ok(result)
    }

    /// Compile to native object code.
    ///
    /// Returns the object file bytes. This does not require a main function.
    pub fn compile(
        &self,
        target: &TargetConfig,
        options: &CodegenOptions,
    ) -> Result<Vec<u8>, CompileError> {
        let mut lowering_result = self.lower_to_execution_graph()?;

        let result = kestrel_codegen_cranelift::compile(&mut lowering_result.mir, target, options)?;

        Ok(result.object_bytes)
    }

    /// Compile and link to an executable.
    ///
    /// Requires a `main` function in the source code.
    pub fn build(
        &self,
        target: &TargetConfig,
        options: &CodegenOptions,
        output: &Path,
    ) -> Result<(), CompileError> {
        let mut lowering_result = self.lower_to_execution_graph()?;

        // Check for main function
        if !self.has_main_function(&lowering_result.mir) {
            return Err(CompileError::NoMainFunction);
        }

        kestrel_codegen_cranelift::compile_and_link(
            &mut lowering_result.mir,
            target,
            options,
            output,
        )?;

        Ok(())
    }

    /// Compile, link, and run the program.
    ///
    /// Requires a `main` function in the source code.
    /// Returns the exit code, stdout, and stderr.
    pub fn run(
        &self,
        target: &TargetConfig,
        options: &CodegenOptions,
    ) -> Result<RunResult, CompileError> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        // Create a unique temp directory for this run
        let temp_dir = std::env::temp_dir().join(format!(
            "kestrel_run_{}_{:?}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            std::thread::current().id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&temp_dir)?;

        // Determine executable name
        let exe_name = if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        };
        let exe_path = temp_dir.join(exe_name);

        // Build the executable
        self.build(target, options, &exe_path)?;

        // Run the executable
        let output = Command::new(&exe_path)
            .output()
            .map_err(|e| CompileError::ExecutionFailed(e.to_string()))?;

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);

        Ok(RunResult::new(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    /// Check if the MIR contains a main function.
    fn has_main_function(&self, mir: &kestrel_execution_graph::MirContext) -> bool {
        for (_, func_def) in mir.functions.iter() {
            let name = mir.name(func_def.name);
            if name.segments.last().map(|s| s.as_str()) == Some("main") {
                return true;
            }
        }
        false
    }
}

/// Lex error diagnostic
struct LexError {
    span: Span,
}

impl IntoDiagnostic for LexError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message("invalid token")
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("unrecognized token"),
            ])
    }
}

/// Parse error diagnostic
struct ParseErrorDiagnostic {
    message: String,
    span: Option<Span>,
}

impl IntoDiagnostic for ParseErrorDiagnostic {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        let mut diagnostic = Diagnostic::error().with_message(&self.message);

        // Add span label if available
        if let Some(span) = &self.span {
            diagnostic = diagnostic.with_labels(vec![
                Label::primary(span.file_id, span.range()).with_message("error occurred here"),
            ]);
        }

        diagnostic
    }
}
