//! # Constraint Cycle Analyzer
//!
//! Detects circular generic constraint dependencies. A cycle occurs when where
//! clause bounds create a dependency loop between type parameters:
//!
//! ```text
//! func foo[A, B]() where A: Protocol[B], B: Protocol[A]
//! //                      ^-- A depends on B, B depends on A
//! ```
//!
//! The algorithm builds a dependency graph from type parameter bounds: if a
//! bound on parameter A references parameter B, then A depends on B. A DFS
//! over this graph detects back-edges (cycles).
//!
//! TODO: This check requires resolved where clause types. The WhereClause
//! component stores AstType subjects and bounds, which need name resolution
//! to identify which type parameters are referenced in each bound.
//!
//! ## Diagnostics
//!
//! ### E451 -- `circular_constraint` (Error, Correctness)
//!
//! **Message:** "circular constraint dependency: '{origin}' -> {cycle}"
//!
//! **Labels:**
//! - Primary: the origin type parameter
//!   - Span source: `util::entity_span` on the type parameter entity
//!   - Message: "cycle starts here"
//! - Secondary: each type parameter in the cycle
//!   - Span source: `util::entity_span` on each participating type parameter entity
//!   - Message: "part of the cycle"
//!
//! **Notes:** (none)

use crate::context::CompilationContext;
use crate::diagnostic::*;
use crate::traits::{CompilationCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E451",
    name: "circular_constraint",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ConstraintCycleAnalyzer;

impl Describe for ConstraintCycleAnalyzer {
    fn id(&self) -> &'static str {
        "constraint_cycles"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl CompilationCheck for ConstraintCycleAnalyzer {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Constraint cycle detection requires:
        // 1. Walk all entities with TypeParams + WhereClause components
        // 2. For each where clause bound, resolve which type parameters it references
        // 3. Build a dependency graph: param_id -> [referenced param_ids]
        // 4. DFS to find cycles in the dependency graph
        //
        // Currently WhereClause stores AstType (unresolved). Need name resolution
        // to identify type parameter references in bounds. Shell returns empty.
        let _ = cx;
        vec![]
    }
}
