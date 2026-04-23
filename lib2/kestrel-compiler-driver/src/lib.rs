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
use kestrel_compiler2::{Compiler, InferWithDiagnostics, diagnostic::WorldFiles};
use kestrel_hecs::Entity;
use kestrel_type_infer::error::InferError;

/// Driver for running whole-program compilation phases on a borrowed `Compiler`.
pub struct CompilerDriver<'a> {
    compiler: &'a Compiler,
}

impl<'a> CompilerDriver<'a> {
    pub fn new(compiler: &'a Compiler) -> Self {
        Self { compiler }
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
                        if let InferError::TypeMismatch { .. } = err {
                            if let Some(detail) = typed.error_details.get(i) {
                                *summary
                                    .type_mismatch_breakdown
                                    .entry(detail.clone())
                                    .or_insert(0) += 1;
                            }
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
    /// cache.
    pub fn analyze_all(&self) -> AnalyzeSummary {
        let world = self.compiler.world();
        let root = self.compiler.root();

        let body_entities: Vec<Entity> = world.iter_component::<Body>().map(|(e, _)| e).collect();
        let decl_entities: Vec<Entity> =
            world.iter_component::<NodeKind>().map(|(e, _)| e).collect();

        let ctx = world.query_context();
        let mut diags = kestrel_analyze::analyze_bodies(&ctx, root, &body_entities);
        diags.extend(kestrel_analyze::analyze_decls(&ctx, root, &decl_entities));
        diags.extend(kestrel_analyze::analyze_compilation(&ctx, root));

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
        summary
    }

    /// Emit all accumulated diagnostics to stderr with source context.
    pub fn emit_diagnostics(&self) -> Result<(), codespan_reporting::files::Error> {
        let diagnostics = self.compiler.diagnostics();
        if diagnostics.is_empty() {
            return Ok(());
        }
        let files = WorldFiles::from_world(self.compiler.world(), self.compiler.files());
        kestrel_reporting2::emit_all(&files, &diagnostics)
    }
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
        InferError::PrimitiveMethodNotCalled { .. } => "PrimitiveMethodNotCalled",
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
        InferError::PrimitiveMethodNotCalled { method, .. } => format!(
            "PrimitiveMethodNotCalled '{}' at {}:{}",
            method, span.file_id, span.start
        ),
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
#[derive(Default)]
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
        let summary = driver.analyze_all();
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
