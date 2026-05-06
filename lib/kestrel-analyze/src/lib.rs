//! kestrel-analyze: Roslyn-inspired analyzer infrastructure for lib.
//!
//! Analyzers are stateless trait objects organized by granularity:
//! - `BodyCheck` — per-function/init body analysis (control flow, types, mutability)
//! - `DeclCheck` — per-declaration structural analysis (conformance, duplicates)
//! - `CompilationCheck` — whole-compilation analysis (cycles)
//!
//! Each analyzer declares descriptors (ID, severity, category) and implements
//! a pure `check` function. The framework handles memoization, routing, and
//! severity configuration.
//!
//! # Query model
//!
//! - `Analyze { analyzer, entity, root }` — run one analyzer on one entity (memoized)
//! - `AnalyzeAll { root }` — run all analyzers on all entities, accumulate diagnostics

pub mod body;
pub mod compilation;
pub mod context;
pub mod decl;
pub mod diagnostic;
pub mod registry;
pub mod traits;
pub mod util;

pub use context::{BodyContext, CompilationContext, DeclContext};
pub use diagnostic::{AnalyzeDiagnostic, Category, DiagLabel, DiagnosticDescriptor, Severity};
pub use registry::{AnalyzerRegistry, AnalyzerRegistryRef};
pub use traits::{BodyCheck, CompilationCheck, DeclCheck, Describe};

use kestrel_ast_builder::NodeKind;
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir_lower::LowerBody;
use kestrel_type_infer::InferBody;

/// Build the default analyzer registry with all built-in analyzers.
pub fn default_analyzers() -> AnalyzerRegistry {
    let mut r = AnalyzerRegistry::new();

    // Body checks
    r.add_body_check(body::exhaustive_return::ExhaustiveReturnAnalyzer);
    r.add_body_check(body::dead_code::DeadCodeAnalyzer);
    r.add_body_check(body::guard::GuardDivergenceAnalyzer);
    r.add_body_check(body::type_check::TypeCheckAnalyzer);
    r.add_body_check(body::condition_check::ConditionCheckAnalyzer);
    r.add_body_check(body::param_pattern::ParamPatternAnalyzer);
    r.add_body_check(body::assignment::AssignmentAnalyzer);

    // Wave 5: Complex body checks
    r.add_body_check(body::definite_assignment::DefiniteAssignmentAnalyzer);
    r.add_body_check(body::initializer::InitializerAnalyzer);
    r.add_body_check(body::closure::ClosureAnalyzer);
    r.add_body_check(body::move_tracking::MoveTrackingAnalyzer);
    r.add_body_check(body::access_mode::AccessModeAnalyzer);

    // Wave 6: Pattern checks
    r.add_body_check(body::refutable_pattern::RefutablePatternAnalyzer);
    r.add_body_check(body::for_loop_pattern::ForLoopPatternAnalyzer);
    r.add_body_check(body::match_pattern::MatchPatternAnalyzer);
    r.add_body_check(body::exhaustiveness::ExhaustivenessAnalyzer);

    // Literal/lexing checks (E700-E799)
    r.add_body_check(body::string_escape::StringEscapeAnalyzer);

    // Declaration checks
    r.add_decl_check(decl::function_body::FunctionBodyAnalyzer);
    r.add_decl_check(decl::protocol_method::ProtocolMethodAnalyzer);
    r.add_decl_check(decl::static_context::StaticContextAnalyzer);
    r.add_decl_check(decl::builtin_marker_protocol::BuiltinMarkerProtocolAnalyzer);
    r.add_decl_check(decl::conformance_rules::ConformanceRulesAnalyzer);
    r.add_decl_check(decl::duplicate_deinit::DuplicateDeinitAnalyzer);
    r.add_decl_check(decl::duplicate_case::DuplicateCaseAnalyzer);
    r.add_decl_check(decl::duplicate_label::DuplicateLabelAnalyzer);
    r.add_decl_check(decl::field::FieldAnalyzer);
    r.add_decl_check(decl::subscript::SubscriptAnalyzer);
    r.add_decl_check(decl::extern_ffi_safe::ExternFfiSafeAnalyzer);
    r.add_decl_check(decl::default_param_ordering::DefaultParamOrderingAnalyzer);
    r.add_decl_check(decl::cloneable_field::CloneableFieldAnalyzer);
    r.add_decl_check(decl::duplicate_symbol::DuplicateSymbolAnalyzer);
    r.add_decl_check(decl::duplicate_callable::DuplicateCallableAnalyzer);
    r.add_decl_check(decl::extension_conflict::ExtensionConflictAnalyzer);
    r.add_decl_check(decl::extension_validation::ExtensionValidationAnalyzer);
    r.add_decl_check(decl::recursive_enum::RecursiveEnumAnalyzer);
    r.add_decl_check(decl::parent_protocol_conformance::ParentProtocolConformanceAnalyzer);
    r.add_decl_check(decl::protocol_field_conformance::ProtocolFieldConformanceAnalyzer);
    r.add_decl_check(decl::generics::GenericsAnalyzer);
    r.add_decl_check(decl::generics::TypeArgArityAnalyzer);
    r.add_decl_check(decl::visibility::VisibilityAnalyzer);
    r.add_decl_check(decl::type_alias_validation::TypeAliasValidationAnalyzer);

    // Compilation checks
    r.add_compilation_check(compilation::type_alias_cycles::TypeAliasCycleAnalyzer);
    r.add_compilation_check(compilation::struct_cycles::StructCycleAnalyzer);
    r.add_compilation_check(compilation::protocol_cycles::ProtocolCycleAnalyzer);
    r.add_compilation_check(compilation::constraint_cycles::ConstraintCycleAnalyzer);
    r.add_compilation_check(compilation::extension_conflict::ExtensionConflictAnalyzer);
    r.add_compilation_check(compilation::conformance_completeness::ConformanceCompletenessAnalyzer);
    r.add_compilation_check(
        compilation::type_annotation_resolution::TypeAnnotationResolutionAnalyzer,
    );
    r.add_compilation_check(compilation::unknown_attribute::UnknownAttributeAnalyzer);

    r
}

// ===== Queries =====

/// Run a single analyzer on a single entity.
///
/// The query key is `(analyzer_id, entity)`, so results are memoized
/// per analyzer per entity. Changing a body only re-runs that body's
/// analyzer queries.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Analyze {
    pub analyzer: String,
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for Analyze {
    type Output = Vec<AnalyzeDiagnostic>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(registry) = ctx.get::<AnalyzerRegistryRef>(self.root) else {
            return vec![];
        };

        // Try body check first
        if let Some(analyzer) = registry.0.find_body_check(&self.analyzer) {
            // Body checks need HIR + typed body
            let Some(hir) = ctx.query(LowerBody {
                entity: self.entity,
                root: self.root,
            }) else {
                return vec![];
            };
            let Some(typed) = ctx.query(InferBody {
                entity: self.entity,
                root: self.root,
            }) else {
                return vec![];
            };

            let cx = BodyContext {
                query: ctx,
                entity: self.entity,
                root: self.root,
                hir: &hir,
                typed: &typed,
            };
            return analyzer.check(&cx);
        }

        // Try decl check
        if let Some(analyzer) = registry.0.find_decl_check(&self.analyzer) {
            let Some(kind) = ctx.get::<NodeKind>(self.entity) else {
                return vec![];
            };
            // Only run if entity kind matches analyzer's target kinds
            if !analyzer.target_kinds().contains(kind) {
                return vec![];
            }
            let cx = DeclContext {
                query: ctx,
                entity: self.entity,
                root: self.root,
                kind: kind.clone(),
            };
            return analyzer.check(&cx);
        }

        vec![]
    }

    fn describe(&self) -> String {
        format!("Analyze({}, {:?})", self.analyzer, self.entity)
    }
}

/// Run all body-check analyzers on a list of body entities.
///
/// Called from the compiler with entities gathered from `World::iter_component::<Body>()`.
/// Each (analyzer, entity) pair dispatches to `Analyze` sub-queries for memoization.
pub fn analyze_bodies(
    ctx: &QueryContext<'_>,
    root: Entity,
    body_entities: &[Entity],
) -> Vec<AnalyzeDiagnostic> {
    let Some(registry) = ctx.get::<AnalyzerRegistryRef>(root) else {
        return vec![];
    };

    let body_ids: Vec<&str> = registry.0.body_checks.iter().map(|a| a.id()).collect();
    let mut all_diags = Vec::new();

    for &entity in body_entities {
        for &analyzer_id in &body_ids {
            let diags = ctx.query(Analyze {
                analyzer: analyzer_id.to_string(),
                entity,
                root,
            });
            all_diags.extend(diags);
        }
    }

    all_diags
}

/// Run all decl-check analyzers on a list of declaration entities.
///
/// Called from the compiler with entities gathered from `World::iter_component::<NodeKind>()`.
/// Each (analyzer, entity) pair dispatches to `Analyze` sub-queries for memoization.
/// The `Analyze` query filters by target_kinds, so entities that don't match an
/// analyzer's targets are skipped cheaply.
pub fn analyze_decls(
    ctx: &QueryContext<'_>,
    root: Entity,
    decl_entities: &[Entity],
) -> Vec<AnalyzeDiagnostic> {
    let Some(registry) = ctx.get::<AnalyzerRegistryRef>(root) else {
        return vec![];
    };

    let decl_ids: Vec<&str> = registry.0.decl_checks.iter().map(|a| a.id()).collect();
    let mut all_diags = Vec::new();

    for &entity in decl_entities {
        for &analyzer_id in &decl_ids {
            let diags = ctx.query(Analyze {
                analyzer: analyzer_id.to_string(),
                entity,
                root,
            });
            all_diags.extend(diags);
        }
    }

    all_diags
}

/// Run all compilation-check analyzers once over the whole compilation.
///
/// Called from the compiler after body and decl checks. Compilation checks
/// see all entities (e.g., for cross-entity cycle detection).
pub fn analyze_compilation(ctx: &QueryContext<'_>, root: Entity) -> Vec<AnalyzeDiagnostic> {
    let Some(registry) = ctx.get::<AnalyzerRegistryRef>(root) else {
        return vec![];
    };

    let cx = CompilationContext { query: ctx, root };

    let mut all_diags = Vec::new();
    for analyzer in &registry.0.compilation_checks {
        all_diags.extend(analyzer.check(&cx));
    }

    all_diags
}
