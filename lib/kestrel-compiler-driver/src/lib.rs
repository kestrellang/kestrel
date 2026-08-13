//! kestrel-compiler-driver: "run everything" orchestration on top of `kestrel-compiler`.
//!
//! `Compiler` exposes per-entity queries and low-level building blocks.
//! `CompilerDriver` wraps a borrowed `Compiler` and provides the full-world
//! scans (`infer_all`, `analyze_all`) plus terminal diagnostic emission. These
//! are the bits that are convenient for CLIs and test harnesses but that a
//! library embedder (LSP, IDE) wouldn't want imposed on it.

use std::collections::HashMap;
use std::fmt;
use std::panic::AssertUnwindSafe;

use kestrel_ast_builder::{Body, Name, NodeKind};
use kestrel_compiler::{Compiler, InferWithDiagnostics, diagnostic::WorldFiles};
use kestrel_hecs::Entity;
use kestrel_type_infer::error::InferError;

/// Driver for running whole-program compilation phases on a borrowed `Compiler`.
///
/// # The two halves of "the diagnostics of this compilation"
///
/// Every phase except analysis deposits into the hECS accumulator, which
/// `Compiler::diagnostics()` reads. Analyzers instead *return* a
/// `Vec<AnalyzeDiagnostic>` on their `AnalyzeSummary`. That second home is why
/// `kestrel dump` printed nothing and exited 0 for a file whose only error was
/// an analyzer code (F17): the caller has to remember both, and one caller
/// didn't.
///
/// The driver now remembers for them. `analyze_all` records its summary, and
/// `emit_diagnostics` / `has_errors` read **both** halves, so no consumer can
/// see only one. Emission is also idempotent: the accumulator is append-only
/// within a revision, so repeated calls print only what is new (`build` emits
/// once before codegen and again after).
pub struct CompilerDriver<'a> {
    compiler: &'a Compiler,
    /// The analyzer half, recorded by `analyze_all`. `None` until analysis runs
    /// — an un-analyzed compilation legitimately has no analyzer diagnostics.
    analyze: std::cell::RefCell<Option<AnalyzeSummary>>,
    /// How many accumulator diagnostics `emit_diagnostics` has already printed.
    emitted_accumulated: std::cell::Cell<usize>,
    /// Whether the analyzer half has already been printed.
    emitted_analyze: std::cell::Cell<bool>,
}

impl<'a> CompilerDriver<'a> {
    pub fn new(compiler: &'a Compiler) -> Self {
        Self {
            compiler,
            analyze: std::cell::RefCell::new(None),
            emitted_accumulated: std::cell::Cell::new(0),
            emitted_analyze: std::cell::Cell::new(false),
        }
    }

    /// Run type inference on every entity with a `Body` component.
    ///
    /// Each body is queried through `InferWithDiagnostics`, so per-body
    /// results are memoized by the query cache. The outer scan is not
    /// incremental — it visits every `Body` entity every call.
    ///
    /// Panics in the solver are caught per-body and recorded in the
    /// summary so one bad body doesn't abort the whole run.
    pub fn infer_all(&self) -> InferSummary {
        let world = self.compiler.world();
        let root = self.compiler.root();

        let entities: Vec<Entity> = world.iter_component::<Body>().map(|(e, _)| e).collect();

        let ctx = world.query_context();
        let mut summary = InferSummary::default();

        for entity in entities {
            summary.total += 1;
            let entity_path = entity_path(self.compiler, entity);

            match std::panic::catch_unwind(AssertUnwindSafe(|| {
                ctx.query(InferWithDiagnostics { entity, root })
            })) {
                Ok(Some(typed)) => {
                    summary.success += 1;
                    summary.errors += typed.errors.len();

                    for (i, err) in typed.errors.iter().enumerate() {
                        let variant = error_variant_name(err);
                        *summary.error_breakdown.entry(variant).or_insert(0) += 1;
                        if let InferError::NoMember { name, .. } = err {
                            *summary.no_member_breakdown.entry(name.clone()).or_insert(0) += 1;
                        }
                        if let InferError::DoesNotConform { protocol, .. } = err {
                            let proto_name = ctx
                                .get::<Name>(*protocol)
                                .map(|n| n.0.clone())
                                .unwrap_or_else(|| format!("{:?}", protocol));
                            *summary
                                .does_not_conform_breakdown
                                .entry(proto_name)
                                .or_insert(0) += 1;
                        }
                        if let InferError::TypeMismatch { .. } = err
                            && let Some(detail) = typed.error_details.get(i)
                        {
                            *summary
                                .type_mismatch_breakdown
                                .entry(detail.clone())
                                .or_insert(0) += 1;
                        }
                    }

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

                    if !typed.errors.is_empty() {
                        let mut details = Vec::new();
                        if typed.errors.len() >= 15 {
                            for (i, err) in typed.errors.iter().enumerate() {
                                let span_info = format!("{}", err.span().start);
                                let detail = typed
                                    .error_details
                                    .get(i)
                                    .cloned()
                                    .unwrap_or_else(|| format_error(err));
                                details.push(format!("@{} {}", span_info, detail));
                            }
                        }
                        summary
                            .body_error_counts
                            .push((entity_path, typed.errors.len(), details));
                    }
                },
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
                },
            }
        }

        summary
    }

    /// Run all registered analyzers on every body and declaration entity.
    ///
    /// Fires `analyze_bodies`, `analyze_decls`, and `analyze_compilation` in
    /// sequence. Results are memoized per `(analyzer, entity)` in the query
    /// cache. `is_executable` reports whether this compilation is producing a
    /// binary; it gates the entry-point requirement (E618). Pass `true` for
    /// `kestrel build` / execution tests, `false` for libraries, `kestrel
    /// check`, the LSP, and diagnostics tests.
    pub fn analyze_all(&self, is_executable: bool) -> AnalyzeSummary {
        let world = self.compiler.world();
        let root = self.compiler.root();

        let body_entities: Vec<Entity> = world.iter_component::<Body>().map(|(e, _)| e).collect();
        let decl_entities: Vec<Entity> =
            world.iter_component::<NodeKind>().map(|(e, _)| e).collect();

        let ctx = world.query_context();
        let mut diags = kestrel_analyze::analyze_bodies(&ctx, root, &body_entities);
        diags.extend(kestrel_analyze::analyze_decls(&ctx, root, &decl_entities));
        diags.extend(kestrel_analyze::analyze_compilation(
            &ctx,
            root,
            is_executable,
        ));

        let mut summary = AnalyzeSummary::default();
        for d in &diags {
            match d.severity {
                kestrel_analyze::Severity::Error => summary.errors += 1,
                kestrel_analyze::Severity::Warning => summary.warnings += 1,
                kestrel_analyze::Severity::Info => summary.info += 1,
            }
            *summary.by_check.entry(d.descriptor_id).or_insert(0) += 1;
        }
        summary.diagnostics = diags;
        // Record the analyzer half so `emit_diagnostics` / `has_errors` see it
        // without every caller having to thread the summary back in (F17).
        *self.analyze.borrow_mut() = Some(summary.clone());
        self.emitted_analyze.set(false);
        summary
    }

    /// Analyzer diagnostics that belong on stderr: errors only.
    ///
    /// Warnings and info are reported through the summary, not printed here.
    fn emittable_analyze_errors(summary: &AnalyzeSummary) -> Vec<&kestrel_analyze::AnalyzeDiagnostic> {
        summary
            .diagnostics
            .iter()
            .filter(|d| d.severity == kestrel_analyze::Severity::Error)
            .collect()
    }

    /// Emit every diagnostic of this compilation to stderr with source context
    /// — both the accumulator half (lex, parse, infer, MIR) and the analyzer
    /// half recorded by `analyze_all`.
    ///
    /// Idempotent: only diagnostics not already printed by an earlier call are
    /// emitted, so a caller may flush at several points without duplicating.
    pub fn emit_diagnostics(&self) -> Result<(), codespan_reporting::files::Error> {
        let accumulated = self.compiler.diagnostics();
        let already = self.emitted_accumulated.get().min(accumulated.len());
        let fresh = &accumulated[already..];

        let analyze = self.analyze.borrow();
        let analyzer_diags = match analyze.as_ref() {
            Some(s) if !self.emitted_analyze.get() => Self::emittable_analyze_errors(s),
            _ => Vec::new(),
        };

        if fresh.is_empty() && analyzer_diags.is_empty() {
            return Ok(());
        }

        let files = WorldFiles::from_world(self.compiler.world(), self.compiler.files());
        let mut to_emit: Vec<codespan_reporting::diagnostic::Diagnostic<usize>> = fresh.to_vec();
        to_emit.extend(analyzer_diags.iter().map(|d| analyze_to_codespan(d)));

        self.emitted_accumulated.set(accumulated.len());
        if analyze.is_some() {
            self.emitted_analyze.set(true);
        }
        kestrel_reporting::emit_all(&files, &to_emit)
    }

    /// Does this compilation have an error in **either** diagnostic half?
    ///
    /// The single gate for "should this command fail?". Reading only
    /// `Compiler::diagnostics()` misses every analyzer code (F17).
    pub fn has_errors(&self) -> bool {
        let accumulated = self
            .compiler
            .diagnostics()
            .iter()
            .any(|d| d.severity >= codespan_reporting::diagnostic::Severity::Error);
        accumulated
            || self
                .analyze
                .borrow()
                .as_ref()
                .is_some_and(|s| s.errors > 0)
    }
}

/// Render an analyzer diagnostic as a codespan diagnostic, so both halves
/// print identically. The descriptor id becomes the diagnostic code.
fn analyze_to_codespan(
    d: &kestrel_analyze::AnalyzeDiagnostic,
) -> codespan_reporting::diagnostic::Diagnostic<usize> {
    use codespan_reporting::diagnostic::{Diagnostic, Label};

    let labels = d
        .labels
        .iter()
        .map(|l| {
            let label = if l.is_primary {
                Label::primary(l.span.file_id, l.span.range())
            } else {
                Label::secondary(l.span.file_id, l.span.range())
            };
            label.with_message(&l.message)
        })
        .collect();
    Diagnostic::error()
        .with_code(d.descriptor_id)
        .with_message(&d.message)
        .with_labels(labels)
        .with_notes(d.notes.clone())
}

/// Build a human-readable dotted path for an entity (e.g. "std.core.Bool.init").
fn entity_path(compiler: &Compiler, entity: Entity) -> String {
    let world = compiler.world();
    let root = compiler.root();
    let mut parts = Vec::new();
    let mut current = Some(entity);
    while let Some(e) = current {
        if e == root {
            break;
        }
        if let Some(name) = world.get::<Name>(e) {
            parts.push(name.0.clone());
        }
        current = world.parent_of(e);
    }
    parts.reverse();
    if parts.is_empty() {
        format!("{:?}", entity)
    } else {
        parts.join(".")
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
        InferError::MemberIsStatic { .. } => "MemberIsStatic",
        InferError::NoAssociatedType { .. } => "NoAssociatedType",
        InferError::InfiniteType { .. } => "InfiniteType",
        InferError::FromHir { .. } => "FromHir",
        InferError::ImplicitMemberNotFound { .. } => "ImplicitMemberNotFound",
        InferError::ArgCountMismatch { .. } => "ArgCountMismatch",
        InferError::LabelMismatch { .. } => "LabelMismatch",
        InferError::InstanceMethodAsStatic { .. } => "InstanceMethodAsStatic",
        InferError::TypeParamAsValue { .. } => "TypeParamAsValue",
        InferError::TypeArgCountMismatch { .. } => "TypeArgCountMismatch",
        InferError::NoMatchingOverload { .. } => "NoMatchingOverload",
        InferError::MemberwiseInitArity { .. } => "MemberwiseInitArity",
        InferError::MemberwiseInitLabel { .. } => "MemberwiseInitLabel",
        InferError::ItWrongArity { .. } => "ItWrongArity",
        InferError::LiteralNotAccepted { .. } => "LiteralNotAccepted",
        InferError::UnresolvedTypeParam { .. } => "UnresolvedTypeParam",
        InferError::CannotInferType { .. } => "CannotInferType",
        InferError::TupleIndexOnNonTuple { .. } => "TupleIndexOnNonTuple",
        InferError::TupleIndexOutOfBounds { .. } => "TupleIndexOutOfBounds",
        InferError::MemberAccessOnPrimitive { .. } => "MemberAccessOnPrimitive",
        InferError::MethodNotCalled { .. } => "MethodNotCalled",
        InferError::CircularOpaqueReturn { .. } => "CircularOpaqueReturn",
        InferError::OpaqueUnderlierNotCopyable { .. } => "OpaqueUnderlierNotCopyable",
        InferError::ConventionMismatch { .. } => "ConventionMismatch",
        InferError::KindMismatch { .. } => "KindMismatch",
        InferError::RefFunctionAsValue { .. } => "RefFunctionAsValue",
        InferError::RefInTypeArgument { .. } => "RefInTypeArgument",
    }
}

/// Format an InferError into a human-readable one-liner.
fn format_error(err: &InferError) -> String {
    let span = err.span();
    match err {
        InferError::TypeMismatch { .. } => {
            format!("TypeMismatch at {}:{}", span.file_id, span.start)
        },
        InferError::DoesNotConform { .. } => {
            format!("DoesNotConform at {}:{}", span.file_id, span.start)
        },
        InferError::NoMember { name, .. } => {
            format!("NoMember '{}' at {}:{}", name, span.file_id, span.start)
        },
        InferError::AmbiguousMember { name, .. } => {
            format!(
                "AmbiguousMember '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::MemberNotVisible { name, .. } => {
            format!(
                "MemberNotVisible '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::MemberIsStatic { name, .. } => {
            format!(
                "MemberIsStatic '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::NoAssociatedType { name, .. } => {
            format!(
                "NoAssociatedType '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::InfiniteType { .. } => {
            format!("InfiniteType at {}:{}", span.file_id, span.start)
        },
        InferError::FromHir { .. } => {
            format!("FromHir at {}:{}", span.file_id, span.start)
        },
        InferError::ImplicitMemberNotFound { name, .. } => {
            format!(
                "ImplicitMemberNotFound '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::ArgCountMismatch { expected, got, .. } => {
            format!(
                "ArgCountMismatch expected={} got={} at {}:{}",
                expected, got, span.file_id, span.start
            )
        },
        InferError::LabelMismatch { expected, got, .. } => {
            format!(
                "LabelMismatch expected={:?} got={:?} at {}:{}",
                expected, got, span.file_id, span.start
            )
        },
        InferError::InstanceMethodAsStatic { name, .. } => {
            format!(
                "InstanceMethodAsStatic '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::TypeParamAsValue { .. } => {
            format!("TypeParamAsValue at {}:{}", span.file_id, span.start)
        },
        InferError::TypeArgCountMismatch { expected, got, .. } => {
            format!(
                "TypeArgCountMismatch expected={} got={} at {}:{}",
                expected, got, span.file_id, span.start
            )
        },
        InferError::NoMatchingOverload { name, .. } => {
            format!(
                "NoMatchingOverload '{}' at {}:{}",
                name, span.file_id, span.start
            )
        },
        InferError::MemberwiseInitArity {
            struct_name,
            expected,
            got,
            ..
        } => format!(
            "MemberwiseInitArity '{}' expected={} got={} at {}:{}",
            struct_name, expected, got, span.file_id, span.start
        ),
        InferError::MemberwiseInitLabel {
            struct_name,
            expected,
            got,
            ..
        } => format!(
            "MemberwiseInitLabel '{}' expected={} got={:?} at {}:{}",
            struct_name, expected, got, span.file_id, span.start
        ),
        InferError::ItWrongArity { expected, .. } => {
            format!(
                "ItWrongArity expected={} at {}:{}",
                expected, span.file_id, span.start
            )
        },
        InferError::LiteralNotAccepted { literal, .. } => {
            format!(
                "LiteralNotAccepted {:?} at {}:{}",
                literal, span.file_id, span.start
            )
        },
        InferError::UnresolvedTypeParam { .. } => {
            format!("UnresolvedTypeParam at {}:{}", span.file_id, span.start)
        },
        InferError::CannotInferType { .. } => {
            format!("CannotInferType at {}:{}", span.file_id, span.start)
        },
        InferError::TupleIndexOnNonTuple { index, .. } => format!(
            "TupleIndexOnNonTuple index={} at {}:{}",
            index, span.file_id, span.start
        ),
        InferError::TupleIndexOutOfBounds { arity, index, .. } => format!(
            "TupleIndexOutOfBounds arity={} index={} at {}:{}",
            arity, index, span.file_id, span.start
        ),
        InferError::MemberAccessOnPrimitive { name, .. } => format!(
            "MemberAccessOnPrimitive '{}' at {}:{}",
            name, span.file_id, span.start
        ),
        InferError::MethodNotCalled { method, .. } => format!(
            "PrimitiveMethodNotCalled '{}' at {}:{}",
            method, span.file_id, span.start
        ),
        InferError::CircularOpaqueReturn { .. } => {
            format!("CircularOpaqueReturn at {}:{}", span.file_id, span.start)
        },
        InferError::OpaqueUnderlierNotCopyable { .. } => {
            format!(
                "OpaqueUnderlierNotCopyable at {}:{}",
                span.file_id, span.start
            )
        },
        InferError::ConventionMismatch { .. } => {
            format!("ConventionMismatch at {}:{}", span.file_id, span.start)
        },
        InferError::KindMismatch {
            expected, actual, ..
        } => {
            format!(
                "KindMismatch (expected {expected:?}, got {actual:?}) at {}:{}",
                span.file_id, span.start
            )
        },
        InferError::RefFunctionAsValue { .. } => {
            format!("RefFunctionAsValue at {}:{}", span.file_id, span.start)
        },
        InferError::RefInTypeArgument { .. } => {
            format!("RefInTypeArgument at {}:{}", span.file_id, span.start)
        },
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

/// Summary of analysis results across all bodies.
#[derive(Clone, Default)]
pub struct AnalyzeSummary {
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    /// Count per descriptor ID (e.g., "E001" → 3).
    pub by_check: HashMap<&'static str, usize>,
    /// All diagnostics produced.
    pub diagnostics: Vec<kestrel_analyze::AnalyzeDiagnostic>,
}

impl fmt::Display for AnalyzeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Analysis Summary:")?;
        writeln!(f, "  Errors:   {}", self.errors)?;
        writeln!(f, "  Warnings: {}", self.warnings)?;
        if self.info > 0 {
            writeln!(f, "  Info:     {}", self.info)?;
        }
        if !self.by_check.is_empty() {
            writeln!(f)?;
            let mut checks: Vec<_> = self.by_check.iter().collect();
            checks.sort_by(|a, b| b.1.cmp(a.1));
            for (id, count) in checks {
                writeln!(f, "    {:20} {:>5}", id, count)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Path to the stdlib directory (relative to workspace root).
    fn stdlib_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    #[test]
    fn compile_simple_function() {
        let mut c = Compiler::new();
        let f = c.set_source(
            "test.ks",
            "module Test\nfunc foo() { let x = 42; x }".into(),
        );
        c.build(f);

        let summary = CompilerDriver::new(&c).infer_all();
        eprintln!("{}", summary);
        assert!(summary.total > 0, "should have at least one body");
        assert_eq!(summary.panics, 0, "simple function should not panic");
    }

    #[test]
    fn compile_full_stdlib() {
        let mut c = Compiler::new();
        c.load_dir(&stdlib_path());

        let summary = CompilerDriver::new(&c).infer_all();
        eprintln!("{}", summary);
        assert!(summary.total > 0, "should have found bodies in stdlib");
    }

    #[test]
    fn analyze_full_stdlib() {
        let mut c = Compiler::new();
        c.load_dir(&stdlib_path());

        let driver = CompilerDriver::new(&c);
        let _infer = driver.infer_all();
        let summary = driver.analyze_all(false);
        eprintln!("{}", summary);
    }

    #[test]
    fn compile_stdlib_bool() {
        let mut c = Compiler::new();
        let core_path = stdlib_path().join("core");
        c.load_dir(&core_path);

        let summary = CompilerDriver::new(&c).infer_all();
        eprintln!("=== Bool + core ===");
        eprintln!("{}", summary);
        assert!(summary.total > 0);
    }
}
