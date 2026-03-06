//! kestrel-compiler2: ECS-driven compiler pipeline built on kestrel-hecs.
//!
//! Source files are entities, compilation phases are queries. Diagnostics
//! are accumulated as side-effects during query execution.
//!
//! # Usage
//!
//! ```
//! use kestrel_compiler2::Compiler;
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
pub use diagnostic::{Diagnostic, Severity};
pub use kestrel_ast_builder;
pub use queries::{LexFile, ParseFile};

use std::collections::HashMap;
use std::fmt;
use std::panic::AssertUnwindSafe;
use std::path::Path;

use kestrel_ast_builder::Body;
use kestrel_hecs::{Entity, World};
use kestrel_lexer2::SpannedToken;
use kestrel_parser2::ParseResult;
use kestrel_type_infer::error::InferError;
use kestrel_type_infer::InferBody;

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
        Self { world, files: HashMap::new(), root }
    }

    /// Add or update a source file. Returns the entity handle.
    ///
    /// Call `begin_revision()` before a batch of source updates to
    /// enable change tracking and query invalidation.
    pub fn set_source(&mut self, path: &str, source: String) -> Entity {
        let world = &mut self.world;
        let entity = *self.files.entry(path.to_string()).or_insert_with(|| {
            world.spawn()
        });
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
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.world.accumulated::<Diagnostic>()
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
        );
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
    /// Run type inference on all entities that have bodies.
    /// Returns a summary of how many succeeded, failed, panicked, etc.
    pub fn infer_all(&self) -> InferSummary {
        // Collect entities with Body component (mutation-phase API)
        let entities: Vec<Entity> = self
            .world
            .iter_component::<Body>()
            .map(|(e, _)| e)
            .collect();

        let ctx = self.world.query_context();
        let mut summary = InferSummary::default();

        for entity in entities {
            summary.total += 1;
            let root = self.root;

            // Build entity path for error reporting (e.g. "std.core.Bool.init")
            let entity_path = self.entity_path(entity);

            match std::panic::catch_unwind(AssertUnwindSafe(|| {
                ctx.query(InferBody { entity, root })
            })) {
                Ok(Some(typed)) => {
                    summary.success += 1;
                    summary.errors += typed.errors.len();

                    // Classify each error by variant
                    for (i, err) in typed.errors.iter().enumerate() {
                        let variant = error_variant_name(err);
                        *summary.error_breakdown.entry(variant).or_insert(0) += 1;
                        if let InferError::NoMember { name, .. } = err {
                            *summary.no_member_breakdown.entry(name.clone()).or_insert(0) += 1;
                        }
                        if let InferError::DoesNotConform { protocol, .. } = err {
                            let proto_name = ctx.get::<kestrel_ast_builder::Name>(*protocol)
                                .map(|n| n.0.clone())
                                .unwrap_or_else(|| format!("{:?}", protocol));
                            *summary.does_not_conform_breakdown.entry(proto_name).or_insert(0) += 1;
                        }
                        if let InferError::TypeMismatch { .. } = err {
                            if let Some(detail) = typed.error_details.get(i) {
                                *summary.type_mismatch_breakdown.entry(detail.clone()).or_insert(0) += 1;
                            }
                        }
                    }

                    // Collect samples (first 50 errors with context)
                    if summary.error_samples.len() < 50 {
                        for err in &typed.errors {
                            if summary.error_samples.len() >= 50 {
                                break;
                            }
                            summary.error_samples.push(ErrorSample {
                                entity_path: entity_path.clone(),
                                error: format_error(err),
                            });
                        }
                    }

                    // Track per-body error counts, with ordered detail for top bodies
                    if !typed.errors.is_empty() {
                        let mut details = Vec::new();
                        if typed.errors.len() >= 15 {
                            for (i, err) in typed.errors.iter().enumerate() {
                                let span_info = error_span(err);
                                let detail = typed.error_details.get(i)
                                    .cloned()
                                    .unwrap_or_else(|| format_error(err));
                                details.push(format!("@{} {}", span_info, detail));
                            }
                        }
                        summary.body_error_counts.push((entity_path, typed.errors.len(), details));
                    }
                }
                Ok(None) => summary.skipped += 1,
                Err(panic) => {
                    summary.panics += 1;
                    let msg = panic
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic.downcast_ref::<&str>().copied())
                        .unwrap_or("unknown panic");
                    summary
                        .panic_details
                        .push(format!("{}: {}", entity_path, msg));
                }
            }
        }

        summary
    }

    /// Build a human-readable path for an entity (e.g. "std.core.Bool.init")
    fn entity_path(&self, entity: Entity) -> String {
        let mut parts = Vec::new();
        let mut current = Some(entity);
        while let Some(e) = current {
            if e == self.root {
                break;
            }
            if let Some(name) = self.world.get::<kestrel_ast_builder::Name>(e) {
                parts.push(name.0.clone());
            }
            current = self.world.parent_of(e);
        }
        parts.reverse();
        if parts.is_empty() {
            format!("{:?}", entity)
        } else {
            parts.join(".")
        }
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

/// Summary of type inference results across all bodies.
#[derive(Default)]
pub struct InferSummary {
    /// Total entities with bodies.
    pub total: usize,
    /// Successfully inferred (may still have type errors).
    pub success: usize,
    /// Skipped — no HIR body produced (e.g., missing Body component path).
    pub skipped: usize,
    /// Panicked during inference.
    pub panics: usize,
    /// Total type errors across all successful inferences.
    pub errors: usize,
    /// Error counts by variant name.
    pub error_breakdown: HashMap<&'static str, usize>,
    /// NoMember breakdown by member name.
    pub no_member_breakdown: HashMap<String, usize>,
    /// DoesNotConform breakdown by protocol name.
    pub does_not_conform_breakdown: HashMap<String, usize>,
    /// TypeMismatch breakdown by "expected X got Y" pattern.
    pub type_mismatch_breakdown: HashMap<String, usize>,
    /// Sample errors with entity context.
    pub error_samples: Vec<ErrorSample>,
    /// Details of panics (entity name + message).
    pub panic_details: Vec<String>,
    /// Per-body error counts: (entity_path, error_count, detail_descriptions).
    pub body_error_counts: Vec<(String, usize, Vec<String>)>,
}

/// A single error sample with the entity it came from.
pub struct ErrorSample {
    pub entity_path: String,
    pub error: String,
}

/// Classify an InferError into a variant name for breakdown.
fn error_variant_name(err: &InferError) -> &'static str {
    match err {
        InferError::TypeMismatch { .. } => "TypeMismatch",
        InferError::DoesNotConform { .. } => "DoesNotConform",
        InferError::NoMember { .. } => "NoMember",
        InferError::AmbiguousMember { .. } => "AmbiguousMember",
        InferError::MemberNotVisible { .. } => "MemberNotVisible",
        InferError::NoAssociatedType { .. } => "NoAssociatedType",
        InferError::InfiniteType { .. } => "InfiniteType",
        InferError::FromHir { .. } => "FromHir",
        InferError::ImplicitMemberNotFound { .. } => "ImplicitMemberNotFound",
    }
}

/// Format an InferError into a human-readable one-liner.
/// Extract byte offset from an error span.
fn error_span(err: &InferError) -> String {
    let span = match err {
        InferError::TypeMismatch { span, .. }
        | InferError::DoesNotConform { span, .. }
        | InferError::NoMember { span, .. }
        | InferError::AmbiguousMember { span, .. }
        | InferError::MemberNotVisible { span, .. }
        | InferError::NoAssociatedType { span, .. }
        | InferError::InfiniteType { span }
        | InferError::FromHir { span }
        | InferError::ImplicitMemberNotFound { span, .. } => span,
    };
    format!("{}", span.start)
}

fn format_error(err: &InferError) -> String {
    match err {
        InferError::TypeMismatch { span, .. } => {
            format!("TypeMismatch at {}:{}", span.file_id, span.start)
        }
        InferError::DoesNotConform { span, .. } => {
            format!("DoesNotConform at {}:{}", span.file_id, span.start)
        }
        InferError::NoMember { name, span, .. } => {
            format!("NoMember '{}' at {}:{}", name, span.file_id, span.start)
        }
        InferError::AmbiguousMember { name, span, .. } => {
            format!("AmbiguousMember '{}' at {}:{}", name, span.file_id, span.start)
        }
        InferError::MemberNotVisible { name, span, .. } => {
            format!("MemberNotVisible '{}' at {}:{}", name, span.file_id, span.start)
        }
        InferError::NoAssociatedType { name, span, .. } => {
            format!("NoAssociatedType '{}' at {}:{}", name, span.file_id, span.start)
        }
        InferError::InfiniteType { span } => {
            format!("InfiniteType at {}:{}", span.file_id, span.start)
        }
        InferError::FromHir { span } => {
            format!("FromHir at {}:{}", span.file_id, span.start)
        }
        InferError::ImplicitMemberNotFound { name, span, .. } => {
            format!("ImplicitMemberNotFound '{}' at {}:{}", name, span.file_id, span.start)
        }
    }
}

impl fmt::Display for InferSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Type Inference Summary:")?;
        writeln!(f, "  Total bodies:  {}", self.total)?;
        writeln!(f, "  Success:       {}", self.success)?;
        writeln!(f, "  Skipped:       {}", self.skipped)?;
        writeln!(f, "  Panics:        {}", self.panics)?;
        writeln!(f, "  Type errors:   {}", self.errors)?;

        if !self.error_breakdown.is_empty() {
            writeln!(f)?;
            writeln!(f, "  Error breakdown:")?;
            // Sort by count descending
            let mut breakdown: Vec<_> = self.error_breakdown.iter().collect();
            breakdown.sort_by(|a, b| b.1.cmp(a.1));
            for (variant, count) in &breakdown {
                writeln!(f, "    {:30} {:>5}", variant, count)?;
            }
        }

        if !self.no_member_breakdown.is_empty() {
            writeln!(f)?;
            writeln!(f, "  NoMember breakdown:")?;
            let mut nm: Vec<_> = self.no_member_breakdown.iter().collect();
            nm.sort_by(|a, b| b.1.cmp(a.1));
            for (name, count) in &nm {
                writeln!(f, "    {:30} {:>5}", name, count)?;
            }
        }

        if !self.does_not_conform_breakdown.is_empty() {
            writeln!(f)?;
            writeln!(f, "  DoesNotConform breakdown:")?;
            let mut dc: Vec<_> = self.does_not_conform_breakdown.iter().collect();
            dc.sort_by(|a, b| b.1.cmp(a.1));
            for (name, count) in &dc {
                writeln!(f, "    {:30} {:>5}", name, count)?;
            }
        }

        if !self.type_mismatch_breakdown.is_empty() {
            writeln!(f)?;
            writeln!(f, "  TypeMismatch breakdown (top 30):")?;
            let mut tm: Vec<_> = self.type_mismatch_breakdown.iter().collect();
            tm.sort_by(|a, b| b.1.cmp(a.1));
            for (desc, count) in tm.iter().take(30) {
                writeln!(f, "    {:50} {:>5}", desc, count)?;
            }
        }

        if !self.error_samples.is_empty() {
            writeln!(f)?;
            writeln!(f, "  Error samples (first 50):")?;
            for sample in &self.error_samples {
                writeln!(f, "    [{}] {}", sample.entity_path, sample.error)?;
            }
        }

        if !self.body_error_counts.is_empty() {
            writeln!(f)?;
            writeln!(f, "  Bodies with most errors (top 20):")?;
            let mut bc = self.body_error_counts.clone();
            bc.sort_by(|a, b| b.1.cmp(&a.1));
            for (path, count, details) in bc.iter().take(20) {
                writeln!(f, "    {:60} {:>5}", path, count)?;
                // Show first 5 unique error details for high-error bodies
                if !details.is_empty() {
                    let mut seen = std::collections::HashSet::new();
                    for d in details.iter().take(10) {
                        if seen.insert(d.clone()) {
                            writeln!(f, "      - {}", d)?;
                        }
                    }
                }
            }
        }

        if !self.panic_details.is_empty() {
            writeln!(f)?;
            writeln!(f, "  Panic details (first 10):")?;
            for detail in self.panic_details.iter().take(10) {
                writeln!(f, "    - {}", detail)?;
            }
            if self.panic_details.len() > 10 {
                writeln!(f, "    ... and {} more", self.panic_details.len() - 10)?;
            }
        }
        Ok(())
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
    use kestrel_lexer2::Token;
    use kestrel_syntax_tree2::SyntaxKind;
    use std::path::PathBuf;

    /// Path to the stdlib directory (relative to workspace root).
    fn stdlib_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

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
                "token {:?} has wrong file_id", tok.value
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

        let id_tok = tokens.iter().find(|t| t.value == Token::Identifier).unwrap();
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
            "expected multiple diagnostics, got {}", diags.len()
        );
        assert!(diags.iter().all(|d| d.severity == Severity::Error));
    }

    #[test]
    fn lex_error_diagnostic_has_correct_file_id() {
        // Diagnostic spans carry the entity's file_id
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "`".into());
        let _tokens = c.lex(f);

        let diag = &c.diagnostics()[0];
        assert_eq!(diag.span.file_id, f.index());
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
        assert!(diags.iter().all(|d| d.span.file_id == bad.index()));
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
        let f = c.set_source("t.ks",
            "module Main\nimport Foo\nstruct Bar {}".into());
        let result = c.parse(f);

        assert!(result.errors.is_empty());
        assert_eq!(result.tree.children().count(), 3);
    }

    #[test]
    fn parse_tree_contains_expected_node_kinds() {
        // Verify specific declaration kinds appear in the tree
        let mut c = Compiler::new();
        let f = c.set_source("t.ks",
            "module Main\nstruct Point { var x: Int64 }".into());
        let result = c.parse(f);
        assert!(result.errors.is_empty());

        let child_kinds: Vec<_> = result.tree.children()
            .map(|n| n.kind())
            .collect();
        // SourceFile children are module declaration + struct declaration nodes
        assert!(
            child_kinds.iter().any(|k| *k == SyntaxKind::ModuleDeclaration),
            "expected ModuleDeclaration, got {:?}", child_kinds
        );
        assert!(
            child_kinds.iter().any(|k| *k == SyntaxKind::StructDeclaration),
            "expected StructDeclaration, got {:?}", child_kinds
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
        // Parse error diagnostics carry meaningful span info
        let mut c = Compiler::new();
        let f = c.set_source("t.ks", "struct 123".into());
        let _result = c.parse(f);

        let diags = c.diagnostics();
        // At least one diagnostic should have a non-zero span
        assert!(
            diags.iter().any(|d| d.span.end > d.span.start),
            "expected at least one diagnostic with a non-empty span"
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
        assert!(c.diagnostics().is_empty(), "diagnostics should be cleared after fix");
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
            "expected both lex and parse diagnostics, got {}", diags.len()
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
        assert!(diags.iter().all(|d| d.span.file_id == bad.index()));
    }

    // ================================================================
    // Incremental: only changed files get reparsed
    // ================================================================

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
            "expected only 2 re-executions (lex+parse for changed file), got {}", delta
        );

        // Verify f2 actually got updated results
        let r2 = c.parse(f2);
        assert_eq!(r2.tree.children().count(), 2, "f2 should have module + struct");
    }

    // ================================================================
    // Type inference: smoke tests on stdlib
    // ================================================================

    #[test]
    fn compile_simple_function() {
        // Baseline: inference on a trivial function without stdlib
        let mut c = Compiler::new();
        let f = c.set_source("test.ks", "module Test\nfunc foo() { let x = 42; x }".into());
        c.build(f);

        let summary = c.infer_all();
        eprintln!("{}", summary);
        assert!(summary.total > 0, "should have at least one body");
        assert_eq!(summary.panics, 0, "simple function should not panic");
    }

    #[test]
    fn compile_full_stdlib() {
        // Load the entire stdlib and see how far inference gets
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);

        let summary = c.infer_all();
        eprintln!("{}", summary);

        // Just report — don't assert hard failures since many things
        // are expected to fail at this stage
        assert!(summary.total > 0, "should have found bodies in stdlib");
    }

    #[test]
    fn compile_stdlib_bool() {
        // A specific, small stdlib file
        let mut c = Compiler::new();
        let path = stdlib_path();

        // Load core/bool.ks — needs core/protocols.ks for Equatable etc.
        let core_path = path.join("core");
        c.load_dir(&core_path);

        let summary = c.infer_all();
        eprintln!("=== Bool + core ===");
        eprintln!("{}", summary);
        assert!(summary.total > 0);
    }
}
