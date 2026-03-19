//! # Move Tracking Analyzer (Shell)
//!
//! Tracks non-copyable value moves through control flow and reports
//! use-after-move errors. This is a shell — full implementation requires
//! copy-semantics information on types (Copyable protocol conformance).
//!
//! ## Diagnostics
//!
//! ### KS500 — `use_after_move` (Error, Correctness)
//!
//! **Message:** "use of moved value '{name}'"
//!
//! **Labels:**
//! - Primary: the expression using the moved value
//!   - Span source: `util::expr_span` on the `HirExprId`
//!   - Message: "value used here after move"
//! - Secondary: the expression where the move occurred
//!   - Span source: `util::expr_span` on the move `HirExprId`
//!   - Message: "value moved here"
//!
//! **Notes:** "non-copyable values can only be used once"
//!
//! ### KS501 — `maybe_moved` (Error, Correctness)
//!
//! **Message:** "value '{name}' may have been moved"
//!
//! **Labels:**
//! - Primary: the expression using the potentially moved value
//!   - Span source: `util::expr_span` on the `HirExprId`
//!   - Message: "value used here, but may have been moved"
//! - Secondary: the expression where the potential move occurred
//!   - Span source: `util::expr_span` on the move `HirExprId`
//!   - Message: "value potentially moved here"
//!
//! **Notes:** "value was moved in one branch but not another"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS500",
        name: "use_after_move",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS501",
        name: "maybe_moved",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct MoveTrackingAnalyzer;

impl Describe for MoveTrackingAnalyzer {
    fn id(&self) -> &'static str {
        "move_tracking"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for MoveTrackingAnalyzer {
    fn check(&self, _cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Implement move tracking analysis.
        //
        // Requirements for full implementation:
        // 1. Determine which types are Copyable (conformance query on entities)
        // 2. Track which locals hold non-copyable values
        // 3. Walk expressions to detect moves (passing to function args,
        //    assignment, return)
        // 4. Track move state through control flow:
        //    - If/else: union of moves (if moved in either branch, maybe-moved)
        //    - Match: union of moves across arms
        //    - Loop: moves in body count as moved
        // 5. Report KS500 when a definitely-moved value is used again
        // 6. Report KS501 when a maybe-moved (conditionally moved) value is used
        vec![]
    }
}
