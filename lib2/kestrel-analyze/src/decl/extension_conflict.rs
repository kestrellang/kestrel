//! # Extension Conflict Analyzer
//!
//! Detects conflicts between extension methods and the target type's own
//! methods, and between methods in different extensions of the same type.
//!
//! In lib1 this is a two-phase analyzer (collect during visit, check in
//! finalize) because it needs to cross-reference extensions that share a
//! target type. In lib2's DeclCheck model, each extension entity is checked
//! independently. Full cross-extension conflict detection requires a
//! CompilationCheck or extension target resolution.
//!
//! ## Status: Shell
//!
//! This analyzer requires **extension target resolution** to find the concrete
//! type each extension targets. The available infrastructure provides:
//! - `ExtensionTarget(AstType)` component on extension entities
//! - `ResolveTypePath` to resolve `AstType::Named` to a target entity
//! - `Name` component + `children_of` to enumerate methods in extensions
//!
//! What's still missing:
//! - **Cross-extension aggregation**: DeclCheck runs per-entity, but conflict
//!   detection needs to see ALL extensions of the same target type together.
//!   This should be promoted to a `CompilationCheck` that gathers extensions
//!   by resolved target entity.
//! - **Substitution overlap analysis**: Generic extensions with where clauses
//!   may not actually overlap; determining overlap requires substitution
//!   comparison that isn't available yet.
//! - **Target type method enumeration**: Need to enumerate the target type's
//!   own methods (not just extension methods) to detect struct-vs-extension
//!   conflicts. This requires `VisibleChildrenByName` or similar on the resolved
//!   target entity.
//!
//! Once extension target resolution + CompilationCheck infra is available:
//! 1. Gather all extensions, resolve each `ExtensionTarget` to a type entity
//! 2. Group extensions by target entity
//! 3. For each group, collect method names from all extensions + the target type
//! 4. Detect name collisions, emitting KS411 (struct vs extension) or KS412
//!    (extension vs extension)
//!
//! ## Diagnostics
//!
//! ### KS411 -- `struct_extension_method_conflict` (Error, Correctness)
//!
//! **Message:** "duplicate method '{method_name}': extension cannot redefine struct method"
//!
//! **Labels:**
//! - Primary: the struct method definition
//!   - Span source: `util::entity_span` on the struct's method entity
//!   - Message: "method defined here on struct"
//! - Secondary: the conflicting extension method
//!   - Span source: `util::entity_span` on the extension's method entity
//!   - Message: "conflicting extension method here"
//!
//! **Notes:**
//! - "Extensions cannot define methods that already exist on the struct"
//! - "Consider renaming the extension method or removing it"
//!
//! ### KS412 -- `duplicate_extension_method` (Error, Correctness)
//!
//! **Message:** "duplicate method '{method_name}' in overlapping extensions"
//!
//! **Labels:**
//! - Primary: the first extension method
//!   - Span source: `util::entity_span` on the first extension's method entity
//!   - Message: "first definition here"
//! - Secondary: the conflicting extension method
//!   - Span source: `util::entity_span` on the second extension's method entity
//!   - Message: "conflicting definition here"
//!
//! **Notes:**
//! - "Extensions that overlap must not define methods with the same name unless one is strictly more specific"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS411",
        name: "struct_extension_method_conflict",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS412",
        name: "duplicate_extension_method",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ExtensionConflictAnalyzer;

impl Describe for ExtensionConflictAnalyzer {
    fn id(&self) -> &'static str {
        "extension_conflict"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ExtensionConflictAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Extension]
    }

    fn check(&self, _cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Shell: blocked on cross-extension aggregation (needs CompilationCheck)
        // and extension target resolution.
        // See module doc for what's available and what's still needed.
        vec![]
    }
}
