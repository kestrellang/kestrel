//! # Access Mode Analyzer (Shell)
//!
//! Validates that arguments passed to mutating/consuming parameters meet
//! the required access mode. For example, a `let` binding cannot be passed
//! to a `mutating` parameter since the callee would modify it.
//!
//! This is a shell — full implementation requires access mode info on
//! callable parameters and resolved call targets.
//!
//! ## Diagnostics
//!
//! ### KS203 — `let_to_mutating` (Error, Correctness)
//!
//! **Message:** "cannot pass 'let' binding '{name}' to 'mutating' parameter"
//!
//! **Labels:**
//! - Primary: the argument expression
//!   - Span source: `util::expr_span` on the argument `HirExprId`
//!   - Message: "cannot pass to 'mutating' parameter '{param}'"
//! - Secondary: the binding declaration
//!   - Span source: local span from `HirBody.locals`
//!   - Message: "binding declared as 'let' here"
//!
//! **Notes:** "help: consider declaring as 'var' instead"
//!
//! ### KS204 — `immutable_field_to_mutating` (Error, Correctness)
//!
//! **Message:** "cannot pass immutable field '{name}' to 'mutating' parameter"
//!
//! **Labels:**
//! - Primary: the argument expression
//!   - Span source: `util::expr_span` on the argument `HirExprId`
//!   - Message: "cannot pass to 'mutating' parameter"
//!
//! **Notes:** (none)
//!
//! ### KS205 — `rvalue_to_mutating` (Error, Correctness)
//!
//! **Message:** "cannot pass temporary value to 'mutating' parameter"
//!
//! **Labels:**
//! - Primary: the argument expression
//!   - Span source: `util::expr_span` on the argument `HirExprId`
//!   - Message: "temporary values cannot be mutated"
//!
//! **Notes:** (none)
//!
//! ### KS206 — `let_to_consuming` (Error, Correctness)
//!
//! **Message:** "cannot pass 'let' binding to 'consuming' parameter when binding is used later"
//!
//! **Labels:**
//! - Primary: the argument expression
//!   - Span source: `util::expr_span` on the argument `HirExprId`
//!   - Message: "consumed here"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS203",
        name: "let_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS204",
        name: "immutable_field_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS205",
        name: "rvalue_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS206",
        name: "let_to_consuming",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct AccessModeAnalyzer;

impl Describe for AccessModeAnalyzer {
    fn id(&self) -> &'static str {
        "access_mode"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for AccessModeAnalyzer {
    fn check(&self, _cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // TODO: Implement access mode validation.
        //
        // Requirements for full implementation:
        // 1. For each Call and MethodCall expression, resolve the callee entity
        //    from `typed.resolutions`
        // 2. Read the callee's `Callable` component to get parameter access modes
        //    (ReceiverKind::Borrowing/Mutating/Consuming)
        // 3. For each argument, check if the argument expression is:
        //    a. A local variable — check if it's mutable (`is_mut`)
        //    b. A field access — check if the field is Settable
        //    c. A temporary (call result, literal, etc.) — not mutable
        // 4. Report errors:
        //    - KS203: `let` local passed to mutating param
        //    - KS204: immutable field passed to mutating param
        //    - KS205: temporary value passed to mutating param
        //    - KS206: value passed to consuming param when used later (needs liveness)
        vec![]
    }
}
