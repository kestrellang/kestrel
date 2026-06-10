//! Analyzer traits — one per analysis granularity.
//!
//! Inspired by Roslyn's action registration model, expressed as Rust traits.
//! Analyzers are stateless ZSTs that implement the relevant trait(s).

use kestrel_ast_builder::NodeKind;

use crate::context::{BodyContext, CompilationContext, DeclContext};
use crate::diagnostic::{AnalyzeDiagnostic, DiagnosticDescriptor};

/// Closed set of analyzer identifiers — the `Analyze` query key.
///
/// One variant per registered analyzer; adding an analyzer means adding a
/// variant here, so an unknown ID is a compile error rather than a silent
/// dispatch miss. Grouped by granularity to mirror `default_analyzers()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalyzerId {
    // Body checks
    ExhaustiveReturn,
    DeadCode,
    GuardDivergence,
    TypeCheck,
    ConditionCheck,
    ParamPattern,
    Assignment,
    DefiniteAssignment,
    Initializer,
    Closure,
    AccessMode,
    MoveTracking,
    RefutablePattern,
    ForLoopPattern,
    MatchPattern,
    Exhaustiveness,
    StringEscape,

    // Decl checks
    FunctionBody,
    ProtocolMethod,
    StaticContext,
    BuiltinMarkerProtocol,
    ConformanceRules,
    DuplicateDeinit,
    DuplicateEnumCase,
    DuplicateEnumLabel,
    Field,
    Subscript,
    ExternFfiSafe,
    DefaultParamOrdering,
    CloneableField,
    DuplicateSymbol,
    DuplicateCallable,
    ExtensionConflictDeclStub,
    ExtensionValidation,
    RecursiveEnum,
    IndirectEnum,
    ParentProtocolConformance,
    ProtocolFieldConformance,
    Generics,
    TypeArgArity,
    Visibility,
    TypeAliasValidation,

    // Compilation checks (run via `analyze_compilation`, never the `Analyze` query)
    TypeAliasCycles,
    StructCycles,
    ProtocolCycles,
    ConstraintCycles,
    ExtensionConflict,
    ConformanceCompleteness,
    TypeAnnotationResolution,
    UnknownAttribute,
    EntryPoint,
}

impl AnalyzerId {
    /// Human-readable snake_case name, used in query `describe()` output.
    pub fn as_str(&self) -> &'static str {
        match self {
            AnalyzerId::ExhaustiveReturn => "exhaustive_return",
            AnalyzerId::DeadCode => "dead_code",
            AnalyzerId::GuardDivergence => "guard_divergence",
            AnalyzerId::TypeCheck => "type_check",
            AnalyzerId::ConditionCheck => "condition_check",
            AnalyzerId::ParamPattern => "param_pattern",
            AnalyzerId::Assignment => "assignment",
            AnalyzerId::DefiniteAssignment => "definite_assignment",
            AnalyzerId::Initializer => "initializer",
            AnalyzerId::Closure => "closure",
            AnalyzerId::AccessMode => "access_mode",
            AnalyzerId::MoveTracking => "move_tracking",
            AnalyzerId::RefutablePattern => "refutable_pattern",
            AnalyzerId::ForLoopPattern => "for_loop_pattern",
            AnalyzerId::MatchPattern => "match_pattern",
            AnalyzerId::Exhaustiveness => "exhaustiveness",
            AnalyzerId::StringEscape => "string_escape",
            AnalyzerId::FunctionBody => "function_body",
            AnalyzerId::ProtocolMethod => "protocol_method",
            AnalyzerId::StaticContext => "static_context",
            AnalyzerId::BuiltinMarkerProtocol => "builtin_marker_protocol",
            AnalyzerId::ConformanceRules => "conformance_rules",
            AnalyzerId::DuplicateDeinit => "duplicate_deinit",
            AnalyzerId::DuplicateEnumCase => "duplicate_enum_case",
            AnalyzerId::DuplicateEnumLabel => "duplicate_enum_label",
            AnalyzerId::Field => "field",
            AnalyzerId::Subscript => "subscript",
            AnalyzerId::ExternFfiSafe => "extern_ffi_safe",
            AnalyzerId::DefaultParamOrdering => "default_param_ordering",
            AnalyzerId::CloneableField => "cloneable_field",
            AnalyzerId::DuplicateSymbol => "duplicate_symbol",
            AnalyzerId::DuplicateCallable => "duplicate_callable",
            AnalyzerId::ExtensionConflictDeclStub => "extension_conflict_decl_stub",
            AnalyzerId::ExtensionValidation => "extension_validation",
            AnalyzerId::RecursiveEnum => "recursive_enum",
            AnalyzerId::IndirectEnum => "indirect_enum",
            AnalyzerId::ParentProtocolConformance => "parent_protocol_conformance",
            AnalyzerId::ProtocolFieldConformance => "protocol_field_conformance",
            AnalyzerId::Generics => "generics",
            AnalyzerId::TypeArgArity => "type_arg_arity",
            AnalyzerId::Visibility => "visibility",
            AnalyzerId::TypeAliasValidation => "type_alias_validation",
            AnalyzerId::TypeAliasCycles => "type_alias_cycles",
            AnalyzerId::StructCycles => "struct_cycles",
            AnalyzerId::ProtocolCycles => "protocol_cycles",
            AnalyzerId::ConstraintCycles => "constraint_cycles",
            AnalyzerId::ExtensionConflict => "extension_conflict",
            AnalyzerId::ConformanceCompleteness => "conformance_completeness",
            AnalyzerId::TypeAnnotationResolution => "type_annotation_resolution",
            AnalyzerId::UnknownAttribute => "unknown_attribute",
            AnalyzerId::EntryPoint => "entry_point",
        }
    }
}

impl std::fmt::Display for AnalyzerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Base: every analyzer identifies itself and declares its diagnostics.
pub trait Describe: Send + Sync + 'static {
    /// Unique analyzer identifier (e.g. `AnalyzerId::ExhaustiveReturn`).
    fn id(&self) -> AnalyzerId;

    /// Diagnostic descriptors this analyzer can produce.
    fn descriptors(&self) -> &'static [DiagnosticDescriptor];
}

/// Analyze function/init bodies (Roslyn: RegisterOperationBlockAction).
///
/// Receives the HIR body + type inference results. Used for control flow
/// analysis, type checking, mutability, dead code, etc.
pub trait BodyCheck: Describe {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic>;
}

/// Analyze declarations structurally (Roslyn: RegisterSymbolAction).
///
/// Receives an entity + its ECS components. Used for conformance checking,
/// duplicate detection, cycle detection, visibility, etc.
pub trait DeclCheck: Describe {
    /// Which declaration kinds this analyzer applies to.
    fn target_kinds(&self) -> &'static [NodeKind];

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic>;
}

/// Whole-compilation analysis (Roslyn: RegisterCompilationAction).
///
/// Runs once per compilation over all entities. Used for cross-entity
/// checks like cycle detection across types.
pub trait CompilationCheck: Describe {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic>;
}
