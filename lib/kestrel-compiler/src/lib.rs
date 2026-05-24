//! kestrel-compiler: ECS-driven compiler pipeline built on kestrel-hecs.
//!
//! Source files are entities, compilation phases are queries. Diagnostics
//! are accumulated as side-effects during query execution.
//!
//! # Usage
//!
//! ```
//! use kestrel_compiler::Compiler;
//!
//! let mut compiler = Compiler::new();
//! let file = compiler.set_source("main.ks", "let x = 42".into());
//! let tokens = compiler.lex(file);
//! let tree = compiler.parse(file);
//! let diagnostics = compiler.diagnostics();
//! ```

pub mod components;
pub mod diagnostic;
pub mod queries;

pub use components::{FilePath, SourceText};
pub use diagnostic::ThrowDiagnostic;
pub use kestrel_ast_builder;
pub use kestrel_reporting::{Diagnostic, Label, Severity};
pub use queries::{InferWithDiagnostics, LexFile, ParseFile};

use std::collections::HashMap;
use std::path::Path;

use kestrel_ast_builder::TargetConfig;
use kestrel_hecs::{Entity, World};
use kestrel_lexer::SpannedToken;
use kestrel_parser::ParseResult;

/// Compiler database backed by an ECS world.
///
/// Wraps `World` and provides a high-level API for feeding source files
/// and running compilation queries. The mutation/query phase distinction
/// is handled internally.
pub struct Compiler {
    world: World,
    /// Maps file paths to their entity handles.
    files: HashMap<String, Entity>,
    /// Root module entity — parent of all top-level modules.
    root: Entity,
    /// Compilation target for conditional filtering (@platform, etc.)
    target: TargetConfig,
}

impl Compiler {
    pub fn new() -> Self {
        let mut world = World::new();
        world.begin_revision();
        let root = world.spawn();
        world.set(root, kestrel_ast_builder::NodeKind::Module);
        world.set(root, kestrel_ast_builder::Name("<root>".to_string()));
        // Seed the lang module so lang.* builtins (lang.i64, lang.alloc, etc.) are available
        kestrel_ast_builder::seed_lang_module(&mut world, root);
        // Register default analyzers on the root entity
        let registry = kestrel_analyze::default_analyzers();
        world.set(
            root,
            kestrel_analyze::AnalyzerRegistryRef(std::sync::Arc::new(registry)),
        );
        Self {
            world,
            files: HashMap::new(),
            root,
            target: TargetConfig::host(),
        }
    }

    /// Create a Compiler from a pre-built World snapshot.
    ///
    /// Used by the test suite to clone a cached stdlib world per test.
    /// The snapshot must already have a root module with lang module
    /// seeded and analyzers registered.
    pub fn from_snapshot(world: World, root: Entity, files: HashMap<String, Entity>) -> Self {
        Self {
            world,
            files,
            root,
            target: TargetConfig::host(),
        }
    }

    /// Access the file path → entity mapping.
    pub fn files(&self) -> &HashMap<String, Entity> {
        &self.files
    }

    /// Set the compilation target for conditional filtering.
    pub fn with_target(mut self, target: TargetConfig) -> Self {
        self.target = target;
        self
    }

    /// Add or update a source file. Returns the entity handle.
    ///
    /// Call `begin_revision()` before a batch of source updates to
    /// enable change tracking and query invalidation.
    pub fn set_source(&mut self, path: &str, source: String) -> Entity {
        let world = &mut self.world;
        let entity = *self
            .files
            .entry(path.to_string())
            .or_insert_with(|| world.spawn());
        self.world.set(entity, FilePath(path.to_string()));
        self.world.set(entity, SourceText(source));
        entity
    }

    /// Lex a source file, returning its token stream.
    ///
    /// Results are memoized — repeated calls with unchanged source
    /// return cached tokens without re-lexing.
    pub fn lex(&self, entity: Entity) -> Vec<SpannedToken> {
        let ctx = self.world.query_context();
        ctx.query(LexFile { entity })
    }

    /// Parse a source file, returning the syntax tree and any errors.
    ///
    /// Depends on `lex` — if the source changes, lexing re-runs first,
    /// which then invalidates the parse result.
    pub fn parse(&self, entity: Entity) -> ParseResult {
        let ctx = self.world.query_context();
        ctx.query(ParseFile { entity })
    }

    /// Collect all diagnostics from the current revision.
    ///
    /// Returns codespan-reporting `Diagnostic`s accumulated by lex, parse,
    /// and type inference queries. Use `CompilerDriver::emit_diagnostics()`
    /// (from `kestrel-compiler-driver`) to render them to a terminal.
    pub fn diagnostics(&self) -> Vec<codespan_reporting::diagnostic::Diagnostic<usize>> {
        self.world
            .accumulated::<codespan_reporting::diagnostic::Diagnostic<usize>>()
    }

    /// Begin a new compilation cycle. Call before updating sources.
    pub fn begin_revision(&mut self) {
        self.world.begin_revision();
    }

    /// Access the underlying world for advanced use or testing.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Mutable access to the underlying world.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Parse a source file and build declaration entities in the ECS world.
    ///
    /// Calls `parse()` then walks the CST to create declaration entities
    /// with components. Container declarations (struct, enum, etc.) have
    /// their members as children in the ECS hierarchy.
    pub fn build(&mut self, file_entity: Entity) {
        let result = self.parse(file_entity);
        kestrel_ast_builder::build_declarations(
            &mut self.world,
            file_entity,
            &result.tree,
            self.root,
            Some(&self.target),
        );
    }

    /// Despawn the file entity and every declaration entity owned by it
    /// (anything carrying `FileId(file_entity)`). Drops the path from
    /// the path → entity map so a subsequent `set_source` allocates a
    /// fresh entity. Modules are not despawned — they span files and
    /// stay live for other consumers.
    ///
    /// Used by the LSP worker to "unbuild" a file before re-running
    /// `build()` on its new source. Phase 1 calls this for every user
    /// file on any user-side change; Phase 2 calls it per-file.
    pub fn unbuild_file(&mut self, file_entity: Entity) {
        let owned: Vec<Entity> = self
            .world
            .iter_component::<kestrel_ast_builder::FileId>()
            .filter_map(|(e, fid)| (fid.0 == file_entity).then_some(e))
            .collect();
        for e in owned {
            self.world.despawn(e);
        }
        self.world.despawn(file_entity);
        self.files.retain(|_, e| *e != file_entity);
    }

    /// Root module entity — parent of all top-level modules.
    pub fn root(&self) -> Entity {
        self.root
    }

    /// Number of query executions (not cache hits) across all revisions.
    pub fn query_exec_count(&self) -> u64 {
        self.world.query_exec_count()
    }
}

impl Compiler {
    /// Lower to MIR and run all post-lowering passes.
    ///
    /// Call after inference has been run (e.g. via
    /// `CompilerDriver::infer_all()` from `kestrel-compiler-driver`).
    ///
    /// Pipeline order:
    /// 1. MIR lowering
    /// 2. `kestrel-ownership` — emits `Drop` / `DropIf` (move check + drop
    ///    elaboration). This is the only pass that produces destructor
    ///    statements; lowering itself must never emit them.
    /// 3. Thunk + layout passes
    ///
    /// Set `KESTREL_VERIFY_MIR=1` to run [`MirModule::verify`] at the
    /// pre-drop-elab and post-pass stages for MIR-1. MIR-2 always runs
    /// verification (warn-only). Stage 8 will flip verification to a
    /// hard error.
    pub fn lower_to_mir(&self) -> kestrel_mir::MirModule {
        self.lower_to_mir_with_diagnostics().0
    }

    /// Lower to MIR using the new kestrel-mir-2 representation.
    ///
    /// Runs the full pass pipeline: clone elab → thunk → drop shim →
    /// drop elab → layout → verify.
    pub fn lower_to_mir2(&self) -> Result<kestrel_mir_2::MirModule, kestrel_codegen_cranelift_2::CodegenError> {
        let mut mir = kestrel_mir_lower_2::lower_module(self.world(), self.root());
        let target = kestrel_mir_2::TargetConfig::host_64();
        let mut next_entity = self.world().entity_count() as u32;
        let verify_result =
            kestrel_mir_2::passes::run_pipeline(&mut mir, &target, &mut next_entity);
        if !verify_result.is_ok() {
            let ctx = self.world().query_context();
            for error in &verify_result.errors {
                ctx.accumulate(diagnostic::verify_error_to_diagnostic(
                    error,
                    &mir,
                    self.world(),
                ));
            }
            return Err(kestrel_codegen_cranelift_2::CodegenError::Unsupported(
                format!("MIR verification failed with {} error(s)", verify_result.errors.len()),
            ));
        }
        Ok(mir)
    }

    /// Same as [`Self::lower_to_mir`] but also returns the
    /// [`kestrel_ownership::Diagnostics`] (E500/E501) accumulated by the
    /// move-check pass. Callers that surface user diagnostics (the CLI,
    /// the test harness) use this variant; callers that just need the
    /// MIR module for codegen / inspection can use the plain wrapper.
    pub fn lower_to_mir_with_diagnostics(
        &self,
    ) -> (kestrel_mir::MirModule, kestrel_ownership::Diagnostics) {
        let verify_on = std::env::var_os("KESTREL_VERIFY_MIR").is_some();
        let mut mir = kestrel_mir_lower::lower_module(self.world(), self.root());
        if verify_on {
            kestrel_mir::passes::verify_with_stage(
                &mir,
                kestrel_mir::passes::VerifyStage::PreDropElab,
            )
            .dump_if_errors();
        }
        let diags = kestrel_ownership::run(&mut mir);
        let mir = mir.with_all_passes();
        if verify_on {
            mir.verify().dump_if_errors();
        }
        (mir, diags)
    }

    /// Lower to MIR, run all passes, and compile to native object code.
    ///
    /// Call after inference has been run. Returns the raw object file bytes.
    #[allow(clippy::result_large_err)]
    pub fn compile_to_object(
        &self,
    ) -> Result<Vec<u8>, kestrel_codegen_cranelift::error::CodegenError> {
        let mir = self.lower_to_mir();
        let target = kestrel_codegen::TargetConfig::host();
        let options = kestrel_codegen_cranelift::CodegenOptions::default();
        let result = kestrel_codegen_cranelift::compile(&mir, &target, &options)?;
        Ok(result.object_bytes)
    }

    /// Lower to MIR, run all passes, compile, and link to an executable.
    ///
    /// Call after inference has been run.
    #[allow(clippy::result_large_err)]
    pub fn compile_and_link(
        &self,
        output_path: &Path,
        options: &kestrel_codegen_cranelift::CodegenOptions,
    ) -> Result<(), kestrel_codegen_cranelift::error::CodegenError> {
        let mir = self.lower_to_mir();
        let target = kestrel_codegen::TargetConfig::host();
        kestrel_codegen_cranelift::compile_and_link(&mir, &target, options, output_path)
    }

    /// Lower to MIR-2, monomorphize, and compile to native object code.
    #[allow(clippy::result_large_err)]
    pub fn compile_to_object2(
        &self,
    ) -> Result<Vec<u8>, kestrel_codegen_cranelift_2::CodegenError> {
        let mir = self.lower_to_mir2()?;
        let target_mir2 = kestrel_mir_2::TargetConfig::host_64();
        let mono = self.monomorphize_mir2(mir, &target_mir2)?;
        let target = kestrel_codegen::TargetConfig::host();
        let options = kestrel_codegen_cranelift_2::CodegenOptions::default();
        let result = kestrel_codegen_cranelift_2::compile(&mono, &target, &options)?;
        Ok(result.object_bytes)
    }

    /// Lower to MIR-2, monomorphize, compile, and link to an executable.
    #[allow(clippy::result_large_err)]
    pub fn compile_and_link2(
        &self,
        output_path: &Path,
        options: &kestrel_codegen_cranelift_2::CodegenOptions,
    ) -> Result<(), kestrel_codegen_cranelift_2::CodegenError> {
        let mir = self.lower_to_mir2()?;
        let target_mir2 = kestrel_mir_2::TargetConfig::host_64();
        let mono = self.monomorphize_mir2(mir, &target_mir2)?;
        let target = kestrel_codegen::TargetConfig::host();
        kestrel_codegen_cranelift_2::compile_and_link(&mono, &target, options, output_path)
    }

    /// Monomorphize a MIR-2 module, accumulating any errors as formal
    /// diagnostics. Post-mono verification runs unconditionally and
    /// aborts compilation on failure.
    #[allow(clippy::result_large_err)]
    fn monomorphize_mir2(
        &self,
        mir: kestrel_mir_2::MirModule,
        target: &kestrel_mir_2::TargetConfig,
    ) -> Result<kestrel_mir_2::mono::MonoModule, kestrel_codegen_cranelift_2::CodegenError> {
        let mono = kestrel_mir_2::mono::monomorphize(mir, target).map_err(|errs| {
            let ctx = self.world().query_context();
            for error in &errs {
                ctx.accumulate(diagnostic::mono_error_to_diagnostic(error, self.world()));
            }
            kestrel_codegen_cranelift_2::CodegenError::Unsupported(format!(
                "monomorphization failed with {} error(s)",
                errs.len(),
            ))
        })?;

        let mono_verify = kestrel_mir_2::mono::verify::verify_mono(&mono);
        if !mono_verify.is_ok() {
            let ctx = self.world().query_context();
            for error in &mono_verify.errors {
                ctx.accumulate(diagnostic::mono_verify_error_to_diagnostic(
                    error,
                    &mono,
                    self.world(),
                ));
            }
            return Err(kestrel_codegen_cranelift_2::CodegenError::Unsupported(
                format!("MIR verification failed with {} error(s)", mono_verify.errors.len()),
            ));
        }

        Ok(mono)
    }

    /// Load all .ks files from a directory, parse and build declarations.
    pub fn load_dir(&mut self, path: &Path) {
        let mut files: Vec<_> = Self::collect_ks_files(path);
        files.sort(); // deterministic order

        for file_path in files {
            let source = match std::fs::read_to_string(&file_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let name = file_path.to_string_lossy().to_string();
            let entity = self.set_source(&name, source);
            self.build(entity);
        }
    }

    /// Recursively collect all .ks files from a directory.
    fn collect_ks_files(dir: &Path) -> Vec<std::path::PathBuf> {
        let mut result = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return result;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                result.extend(Self::collect_ks_files(&path));
            } else if path.extension().is_some_and(|e| e == "ks") {
                result.push(path);
            }
        }
        result
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::Token;
    use kestrel_syntax_tree::SyntaxKind;

    /// Helper: extract non-trivia token kinds from a token stream.
    fn structural_tokens(tokens: &[SpannedToken]) -> Vec<Token> {
        tokens
            .iter()
            .filter(|t| !matches!(t.value, Token::Whitespace | Token::Newline))
            .map(|t| t.value.clone())
            .collect()
    }

    // ================================================================
    // Lex: output correctness
    // ================================================================

    #[test]
    fn lex_produces_expected_tokens() {
        // Verify the query returns the right token sequence for a known input
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "let x = 42".into());
        let tokens = structural_tokens(&c.lex(f));

        assert_eq!(
            tokens,
            vec![Token::Let, Token::Identifier, Token::Equals, Token::Integer]
        );
    }

    #[test]
    fn lex_token_spans_use_entity_index_as_file_id() {
        // Entity.index() is used as file_id in all token spans
        let mut c = Compiler::new();
        let f = c.set_source("a.ks", "let x".into());
        let entity_idx = f.index();

        let tokens = c.lex(f);
        for tok in &tokens {
            assert_eq!(
                tok.span.file_id, entity_idx,
                "token {:?} has wrong file_id",
                tok.value
            );
        }
    }

    #[test]
    fn lex_token_spans_cover_source_positions() {
        // Verify byte offsets are plausible for "let x"
        //                                        012 34
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "let x".into());
        let tokens = c.lex(f);

        let let_tok = tokens.iter().find(|t| t.value == Token::Let).unwrap();
        assert_eq!(let_tok.span.start, 0);
        assert_eq!(let_tok.span.end, 3);

        let id_tok = tokens
            .iter()
            .find(|t| t.value == Token::Identifier)
            .unwrap();
        assert_eq!(id_tok.span.start, 4);
        assert_eq!(id_tok.span.end, 5);
    }

    #[test]
    fn lex_empty_source() {
        // Empty source produces no tokens and no diagnostics
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "".into());
        assert!(c.lex(f).is_empty());
        assert!(c.diagnostics().is_empty());
    }

    // ================================================================
    // Lex: error → diagnostic propagation
    // ================================================================

    #[test]
    fn lex_error_becomes_diagnostic() {
        // A single invalid character produces exactly one Error diagnostic
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "`".into());
        let _tokens = c.lex(f);

        let diags = c.diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].message, "unexpected character");
    }

    #[test]
    fn lex_multiple_errors_produce_multiple_diagnostics() {
        // Each invalid character is a separate diagnostic
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "` ~ `".into());
        let _tokens = c.lex(f);

        let diags = c.diagnostics();
        assert!(
            diags.len() >= 2,
            "expected multiple diagnostics, got {}",
            diags.len()
        );
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn lex_error_diagnostic_has_correct_file_id() {
        // Diagnostic labels carry the entity's file_id
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "`".into());
        let _tokens = c.lex(f);

        let diag = &c.diagnostics()[0];
        assert_eq!(diag.labels[0].file_id, f.index());
    }

    #[test]
    fn lex_valid_tokens_returned_alongside_errors() {
        // Good tokens still appear even when some characters are invalid
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "let ` x".into());
        let tokens = structural_tokens(&c.lex(f));

        assert!(tokens.contains(&Token::Let));
        assert!(tokens.contains(&Token::Identifier));
        assert!(!c.diagnostics().is_empty());
    }

    // ================================================================
    // Lex: query mechanics (memoization, invalidation, independence)
    // ================================================================

    #[test]
    fn lex_memoized_within_revision() {
        // Two calls in the same revision return equal results
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "var y = 1".into());

        let tokens1 = c.lex(f);
        let tokens2 = c.lex(f);
        assert_eq!(tokens1.len(), tokens2.len());
    }

    #[test]
    fn lex_invalidated_by_source_change() {
        // Changing source in a new revision produces different tokens
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "let a".into());
        let t1 = structural_tokens(&c.lex(f));

        c.begin_revision();
        c.set_source("t.ks", "var b = 99".into());
        let t2 = structural_tokens(&c.lex(f));

        // Different source → different token sequence
        assert_ne!(t1, t2);
    }

    #[test]
    fn lex_entities_are_independent() {
        // An error in one file doesn't affect another
        let mut c = Compiler::new();
        let good = c.set_source("good.ks", "let x = 1".into());
        let bad = c.set_source("bad.ks", "`".into());

        let good_tokens = c.lex(good);
        let _bad_tokens = c.lex(bad);

        assert!(!good_tokens.is_empty());
        // Only bad file produces diagnostics; diagnostics carry its file_id
        let diags = c.diagnostics();
        assert!(diags.iter().all(|d| d.labels[0].file_id == bad.index()));
    }

    // ================================================================
    // Lex: edge case
    // ================================================================

    #[test]
    fn lex_missing_source_returns_empty() {
        // Entity without SourceText component → empty token vec
        let mut c = Compiler::new();
        let e = c.world.spawn();
        assert!(c.lex(e).is_empty());
        assert!(c.diagnostics().is_empty());
    }

    // ================================================================
    // Parse: output correctness
    // ================================================================

    #[test]
    fn parse_produces_source_file_root() {
        // Any parse through the query always yields a SourceFile root node
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module Main".into());
        assert_eq!(c.parse(f).tree.kind(), SyntaxKind::SourceFile);
    }

    #[test]
    fn parse_tree_has_correct_child_count() {
        // Each top-level declaration becomes a child of SourceFile
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module Main\nimport Foo\nstruct Bar {}".into());
        let result = c.parse(f);

        assert!(result.errors.is_empty());
        assert_eq!(result.tree.children().count(), 3);
    }

    #[test]
    fn parse_tree_contains_expected_node_kinds() {
        // Verify specific declaration kinds appear in the tree
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module Main\nstruct Point { var x: Int64 }".into());
        let result = c.parse(f);
        assert!(result.errors.is_empty());

        let child_kinds: Vec<_> = result.tree.children().map(|n| n.kind()).collect();
        // SourceFile children are module declaration + struct declaration nodes
        assert!(
            child_kinds
                .iter()
                .any(|k| *k == SyntaxKind::ModuleDeclaration),
            "expected ModuleDeclaration, got {:?}",
            child_kinds
        );
        assert!(
            child_kinds
                .iter()
                .any(|k| *k == SyntaxKind::StructDeclaration),
            "expected StructDeclaration, got {:?}",
            child_kinds
        );
    }

    #[test]
    fn parse_empty_source() {
        // Empty string → SourceFile with no children, no errors
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "".into());
        let result = c.parse(f);

        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
        assert_eq!(result.tree.children().count(), 0);
        assert!(result.errors.is_empty());
        assert!(c.diagnostics().is_empty());
    }

    // ================================================================
    // Parse: error → diagnostic propagation
    // ================================================================

    #[test]
    fn parse_error_becomes_diagnostic() {
        // Incomplete declaration → at least one Error diagnostic
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module".into());
        let _result = c.parse(f);

        let diags = c.diagnostics();
        assert!(!diags.is_empty(), "parse errors should produce diagnostics");
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn parse_error_diagnostic_has_span() {
        // Parse error diagnostics carry meaningful label info
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "struct 123".into());
        let _result = c.parse(f);

        let diags = c.diagnostics();
        // At least one diagnostic should have a non-empty label range
        assert!(
            diags
                .iter()
                .any(|d| !d.labels.is_empty() && d.labels[0].range.end > d.labels[0].range.start),
            "expected at least one diagnostic with a non-empty label range"
        );
    }

    #[test]
    fn parse_still_produces_tree_on_error() {
        // Error recovery: parser builds a tree even with syntax errors
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module".into());
        let result = c.parse(f);

        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
    }

    // ================================================================
    // Parse: query mechanics
    // ================================================================

    #[test]
    fn parse_memoized_within_revision() {
        // Two calls in the same revision return structurally equal results
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module A\nstruct B {}".into());

        let r1 = c.parse(f);
        let r2 = c.parse(f);
        assert_eq!(r1.tree.children().count(), r2.tree.children().count());
        assert_eq!(r1.errors.len(), r2.errors.len());
    }

    #[test]
    fn parse_invalidated_by_source_change() {
        // Source change → lex invalidation → parse invalidation
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module A".into());
        let r1 = c.parse(f);

        c.begin_revision();
        c.set_source("t.ks", "module A\nstruct B {}\nstruct C {}".into());
        let r2 = c.parse(f);

        assert!(r2.tree.children().count() > r1.tree.children().count());
    }

    #[test]
    fn parse_diagnostics_cleared_on_recompute() {
        // Fix a broken file → stale parse diagnostics disappear
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module".into());
        let _r1 = c.parse(f);
        assert!(!c.diagnostics().is_empty(), "should have errors initially");

        c.begin_revision();
        c.set_source("t.ks", "module Main".into());
        let r2 = c.parse(f);
        assert!(r2.errors.is_empty());
        assert!(
            c.diagnostics().is_empty(),
            "diagnostics should be cleared after fix"
        );
    }

    // ================================================================
    // Parse: edge case
    // ================================================================

    #[test]
    fn parse_missing_source_returns_empty_tree() {
        // Entity without SourceText → empty SourceFile, no errors
        let mut c = Compiler::new();
        let e = c.world.spawn();
        let result = c.parse(e);

        assert_eq!(result.tree.kind(), SyntaxKind::SourceFile);
        assert_eq!(result.tree.children().count(), 0);
        assert!(c.diagnostics().is_empty());
    }

    // ================================================================
    // Cross-query: lex + parse interaction
    // ================================================================

    #[test]
    fn lex_and_parse_errors_both_accumulated() {
        // Source with both invalid characters and bad syntax
        // produces diagnostics from both lex and parse phases
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "` struct".into());
        let _result = c.parse(f);

        let diags = c.diagnostics();
        // At least one lex error ("unexpected character") and one parse error
        assert!(
            diags.iter().any(|d| d.message == "unexpected character"),
            "expected a lex error diagnostic"
        );
        assert!(
            diags.len() >= 2,
            "expected both lex and parse diagnostics, got {}",
            diags.len()
        );
    }

    #[test]
    fn parse_only_invocation_still_produces_lex_diagnostics() {
        // Calling parse() (not lex()) still surfaces lex errors,
        // because parse depends on lex internally
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "let ` x = 1".into());
        let _result = c.parse(f);

        let diags = c.diagnostics();
        assert!(
            diags.iter().any(|d| d.message == "unexpected character"),
            "lex diagnostic should surface through parse"
        );
    }

    #[test]
    fn parse_and_lex_independent_across_entities() {
        // Errors in one entity don't leak into another's results
        let mut c = Compiler::new();
        let good = c.set_source("good.ks", "module Main\nstruct A {}".into());
        let bad = c.set_source("bad.ks", "` module".into());

        let good_result = c.parse(good);
        let _bad_result = c.parse(bad);

        assert!(good_result.errors.is_empty());
        // All diagnostics belong to the bad file
        let diags = c.diagnostics();
        assert!(diags.iter().all(|d| d.labels[0].file_id == bad.index()));
    }

    // ================================================================
    // Incremental: only changed files get reparsed
    // ================================================================

    // ================================================================
    // unbuild_file: incremental teardown
    // ================================================================

    #[test]
    fn unbuild_file_removes_file_and_owned_decls() {
        // Build a file with a struct, then unbuild it. The file entity
        // and the struct's declaration entity should be despawned.
        use kestrel_ast_builder::FileId;
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "module M\nstruct Foo {}".into());
        c.build(f);

        // Find the struct decl by FileId.
        let owned_before: Vec<_> = c
            .world
            .iter_component::<FileId>()
            .filter(|(_, fid)| fid.0 == f)
            .map(|(e, _)| e)
            .collect();
        assert!(
            !owned_before.is_empty(),
            "expected at least one owned decl after build"
        );

        c.unbuild_file(f);

        assert!(!c.world.is_alive(f), "file entity should be despawned");
        for e in &owned_before {
            assert!(
                !c.world.is_alive(*e),
                "owned decl {:?} should be despawned",
                e
            );
        }
        assert!(
            c.files.get("t.ks").is_none(),
            "path entry should be dropped"
        );
    }

    #[test]
    fn unbuild_file_does_not_disturb_other_files() {
        use kestrel_ast_builder::FileId;
        let mut c = Compiler::new();
        let a = c.set_source("a.ks", "module A\nstruct AA {}".into());
        let b = c.set_source("b.ks", "module B\nstruct BB {}".into());
        c.build(a);
        c.build(b);

        let b_decls: Vec<_> = c
            .world
            .iter_component::<FileId>()
            .filter(|(_, fid)| fid.0 == b)
            .map(|(e, _)| e)
            .collect();

        c.unbuild_file(a);

        assert!(c.world.is_alive(b));
        for e in &b_decls {
            assert!(
                c.world.is_alive(*e),
                "b's decl {:?} should still be alive",
                e
            );
        }
    }

    #[test]
    fn rebuild_after_unbuild_yields_new_entities() {
        let mut c = Compiler::new();
        let f1 = c.set_source("t.ks", "module M\nstruct Foo {}".into());
        c.build(f1);

        c.unbuild_file(f1);
        c.begin_revision();

        let f2 = c.set_source("t.ks", "module M\nstruct Bar {}".into());
        c.build(f2);

        assert_ne!(f1, f2, "rebuild must allocate a new file entity");
        assert!(c.world.is_alive(f2));
        // Parse should reflect the new source.
        let r = c.parse(f2);
        assert!(
            r.tree
                .children()
                .any(|n| n.kind() == kestrel_syntax_tree::SyntaxKind::StructDeclaration)
        );
    }

    #[test]
    fn only_changed_file_gets_reparsed() {
        // Parse 3 files, change one, verify only that file re-executes
        // its lex + parse queries while the others are served from cache.
        let mut c = Compiler::new();
        let f1 = c.set_source("a.ks", "module A".into());
        let f2 = c.set_source("b.ks", "module B".into());
        let f3 = c.set_source("c.ks", "module C".into());

        // Rev 1: parse all 3 files
        c.parse(f1);
        c.parse(f2);
        c.parse(f3);

        // 3 LexFile + 3 ParseFile = 6 query executions
        let after_rev1 = c.query_exec_count();
        assert_eq!(after_rev1, 6, "expected 6 executions in rev1");

        // Rev 2: change only f2
        c.begin_revision();
        c.set_source("b.ks", "module B\nstruct Foo {}".into());

        c.parse(f1);
        c.parse(f2);
        c.parse(f3);

        // Only f2's LexFile + ParseFile should re-execute = 2 new executions
        let delta = c.query_exec_count() - after_rev1;
        assert_eq!(
            delta, 2,
            "expected only 2 re-executions (lex+parse for changed file), got {}",
            delta
        );

        // Verify f2 actually got updated results
        let r2 = c.parse(f2);
        assert_eq!(
            r2.tree.children().count(),
            2,
            "f2 should have module + struct"
        );
    }
}
