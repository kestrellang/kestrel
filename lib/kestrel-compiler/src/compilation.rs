use crate::source_file::SourceFile;
use kestrel_lexer::lex;
use kestrel_parser::{Parser, parse_source_file};
use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_analyzers::analyzers::DuplicateSymbolAnalyzer;
use kestrel_semantic_analyzers::{AnalysisContext, Analyzer, run_all};
use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree_binder::SemanticBinder;
use kestrel_semantic_tree_builder::SemanticModelBuilder;
use kestrel_span::Span;

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
            let parse_result = Parser::parse(&source, tokens.into_iter(), parse_source_file);

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
                &mut diagnostics,
                file_id,
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
