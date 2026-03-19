//! # For-Loop Pattern Analyzer (Shell)
//!
//! Checks that for-loop bindings use irrefutable patterns. In lib2 HIR,
//! for-loops are desugared to loop + match on iterator.next(), so the
//! user's pattern is embedded inside the desugared match. This analyzer
//! would need to recognize the desugaring pattern to extract and validate
//! the user's original pattern.
//!
//! Currently a shell — the desugaring produces irrefutable bindings
//! by construction, so this check may not fire in practice. If the HIR
//! adds a `from_for_loop` marker (like lib1's `WhileLet::from_for_loop`),
//! this can be properly implemented.
//!
//! ## Diagnostics
//!
//! ### KS301 — `refutable_for_loop_pattern` (Error, Correctness)
//!
//! **Message:** "refutable pattern in for-loop binding"
//!
//! **Labels:**
//! - Primary: the user's pattern inside the desugared for-loop
//!   - Span source: `util::pat_span` on the `HirPatId`
//!   - Message: "this pattern may not match all iterator elements"
//!
//! **Notes:** "help: for-loop patterns must match every element from the iterator"

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS301",
    name: "refutable_for_loop_pattern",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ForLoopPatternAnalyzer;

impl Describe for ForLoopPatternAnalyzer {
    fn id(&self) -> &'static str {
        "for_loop_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ForLoopPatternAnalyzer {
    fn check(&self, _cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Implement for-loop pattern checking.
        //
        // In lib2 HIR, for-loops are desugared to:
        //   var iter = iterable.iter()
        //   loop {
        //     match iter.next() {
        //       .Some(user_pattern) => { body }
        //       .None => break
        //     }
        //   }
        //
        // To check the user's pattern, we'd need to:
        // 1. Identify desugared for-loops (possibly via a marker on HirBody)
        // 2. Extract the user_pattern from inside the .Some variant pattern
        // 3. Check if that pattern is irrefutable using
        //    `refutable_pattern::is_pattern_irrefutable`
        //
        // Since the desugaring currently always produces irrefutable bindings
        // (the user's pattern is used directly), this check is a no-op for now.
        vec![]
    }
}
