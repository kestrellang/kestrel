//! # Type Alias Cycle Analyzer
//!
//! Detects circular type alias dependencies. A cycle occurs when type aliases
//! reference each other, directly or transitively, creating an infinite
//! expansion:
//!
//! ```text
//! type A = B      // direct: A -> B -> A
//! type B = A
//!
//! type X = (Y, Z) // transitive: X -> Y -> X
//! type Y = X
//! ```
//!
//! The cycle walk follows type alias references through tuples and function
//! types, but stops at struct/enum/protocol types (which introduce indirection).
//!
//! TODO: This check requires resolved aliased types. Currently the ECS stores
//! `TypeAnnotation(AstType)` on type aliases, which is unresolved. Cycle detection
//! needs to follow alias definitions to their resolved type and check if any
//! referenced alias has already been visited.
//!
//! ## Diagnostics
//!
//! ### KS447 -- `circular_type_alias` (Error, Correctness)
//!
//! **Message:** "circular type alias: '{origin}' references itself through {cycle}"
//!
//! **Labels:**
//! - Primary: the origin type alias
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "cycle starts here"
//! - Secondary: each participant in the cycle
//!   - Span source: `util::entity_span` on each participating type alias entity
//!   - Message: "part of the cycle"
//!
//! **Notes:** (none)
//!
//! ### KS448 -- `type_alias_contains_infer` (Warning, Correctness)
//!
//! **Message:** "type alias '{name}' contains an unresolved type"
//!
//! **Labels:**
//! - Primary: the type alias declaration
//!   - Span source: `util::entity_span` on the type alias entity
//!   - Message: "type could not be fully resolved"
//!
//! **Notes:** (none)

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS447",
        name: "circular_type_alias",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS448",
        name: "type_alias_contains_infer",
        default_severity: Severity::Warning,
        category: Category::Correctness,
    },
];

pub struct TypeAliasCycleAnalyzer;

impl Describe for TypeAliasCycleAnalyzer {
    fn id(&self) -> &'static str {
        "type_alias_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for TypeAliasCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Type alias cycle detection requires:
        // 1. Walk all TypeAlias entities in the world
        // 2. For each, resolve its TypeAnnotation to a concrete type
        // 3. Follow type alias references transitively (through tuples, function types)
        // 4. Use a visited set / cycle detector to find back-edges
        //
        // Currently the ECS stores AstType (unresolved) on TypeAlias entities.
        // Cycle detection needs resolved alias targets. Shell returns empty.
        let _ = cx;
        vec![]
    }
}
