//! # Exhaustive Return Analyzer
//!
//! Checks that all code paths in non-unit functions return a value.
//! Skips unit-return functions and empty bodies (protocol decls).
//! Runs the CFG over both statement list and tail expression: a tail
//! expression that does not itself guarantee a return (e.g. an `if`
//! without `else`, a `while`, or a `loop` that can break) leaves the
//! body `MayFallThrough` — which triggers E001 just like a trailing
//! statement would.
//!
//! ## Diagnostics
//!
//! ### E001 — `missing_return` (Error, Correctness)
//!
//! **Message:** "function '{name}' does not return a value on all code paths"
//!
//! **Labels:**
//! - Primary: the function body's closing `}`
//!   - Span source: `util::body_close_brace_span` on the function entity,
//!     falling back to `util::stmt_span` on the last `HirStmtId`
//!   - Message: "missing return"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{NodeKind, TypeAnnotation};
use kestrel_hir::body::*;
use kestrel_type_infer::result::{ResolvedTy, TypedBody};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E001",
    name: "missing_return",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ExhaustiveReturnAnalyzer;

impl Describe for ExhaustiveReturnAnalyzer {
    fn id(&self) -> &'static str {
        "exhaustive_return"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ExhaustiveReturnAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Only check functions (not inits, deinits, etc.)
        let kind = cx.query.get::<NodeKind>(cx.entity);
        if !matches!(kind, Some(NodeKind::Function)) {
            return vec![];
        }

        // Skip unit-return functions — these don't require an explicit
        // return value on every path. Both a missing annotation and an
        // explicit `-> ()` / `-> Tuple()` return type mean unit.
        match cx.query.get::<TypeAnnotation>(cx.entity) {
            None => return vec![],
            Some(ann) if is_unit_ty(&ann.0) => return vec![],
            _ => {},
        }

        // Skip empty bodies (protocol declarations, extern functions)
        if cx.hir.statements.is_empty() && cx.hir.tail_expr.is_none() {
            return vec![];
        }

        // Skip when the body already has type-inference errors. A missing
        // return is usually a secondary symptom in that case — the user
        // needs to fix the root-cause error first, and E001 would spam.
        if !cx.typed.errors.is_empty() {
            return vec![];
        }

        // Run the CFG over the full body (statements + tail expression).
        // If any path can fall through without returning, emit E001.
        let state = block_state(cx.hir, cx.typed, &cx.hir.statements, cx.hir.tail_expr);
        if state.definitely_returns() {
            return vec![];
        }

        let func_name = util::entity_name(cx.query, cx.entity);

        let span = util::body_close_brace_span(cx.query, cx.entity)
            .or_else(|| {
                cx.hir
                    .statements
                    .last()
                    .map(|&id| util::stmt_span(cx.hir, id))
            })
            .unwrap_or_else(|| kestrel_span2::Span::synthetic(0));

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "function '{}' does not return a value on all code paths",
                func_name
            ),
            labels: vec![DiagLabel {
                span,
                message: "missing return".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}

/// Whether an `AstType` names the unit type — either `AstType::Unit` or an
/// empty tuple `()` spelled as `Tuple(vec![])`.
fn is_unit_ty(ty: &AstType) -> bool {
    matches!(ty, AstType::Unit(_)) || matches!(ty, AstType::Tuple(elems, _) if elems.is_empty())
}

// ===== Control flow divergence analysis =====
//
// Uses a 3-state ReturnState to distinguish:
// - Returns: all paths return a value (via `return` or throw)
// - Diverges: control flow exits abnormally (break/continue/infinite loop)
// - MayFallThrough: control may reach the end without returning
//
// For the exhaustive return check, both Returns and Diverges count as
// "definitely returns" — the function won't silently fall off the end.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnState {
    Returns,
    Diverges,
    MayFallThrough,
}

impl ReturnState {
    fn definitely_returns(self) -> bool {
        matches!(self, ReturnState::Returns | ReturnState::Diverges)
    }

    /// Merge two branches (if/else, match arms).
    /// Both branches must definitely return for the whole to definitely return.
    fn merge(self, other: ReturnState) -> ReturnState {
        match (self, other) {
            (ReturnState::Returns, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Returns, ReturnState::Diverges)
            | (ReturnState::Diverges, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Diverges) => ReturnState::Diverges,
            _ => ReturnState::MayFallThrough,
        }
    }
}

/// Whether a block of statements definitely returns.
///
/// Tail-expression handling: if the tail itself definitely returns/diverges,
/// propagate that. Otherwise, distinguish control-flow tails from
/// value-producing tails:
/// - Control-flow (`if`/`match`/`loop`/`block`) that does NOT definitely
///   return is MayFallThrough — a missing `else`, a while-break, etc. can
///   leave the block without a value on the relevant return type.
/// - Everything else (literals, calls, arithmetic, field access, ...) is
///   treated as producing the block's value → `Returns` for the purpose
///   of exhaustive-return analysis.
fn block_state(
    hir: &HirBody,
    typed: &TypedBody,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
) -> ReturnState {
    for &stmt_id in stmts {
        let state = stmt_state(hir, typed, stmt_id);
        if state.definitely_returns() {
            return state;
        }
    }
    let Some(tail) = tail else {
        return ReturnState::MayFallThrough;
    };
    let state = expr_state(hir, typed, tail);
    if state.definitely_returns() {
        return state;
    }
    match &hir.exprs[tail] {
        HirExpr::If { .. }
        | HirExpr::Match { .. }
        | HirExpr::Loop { .. }
        | HirExpr::Block { .. } => state,
        _ => ReturnState::Returns,
    }
}

fn stmt_state(hir: &HirBody, typed: &TypedBody, id: HirStmtId) -> ReturnState {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_state(hir, typed, *expr),
        HirStmt::Let { value: Some(v), .. } => expr_state(hir, typed, *v),
        _ => ReturnState::MayFallThrough,
    }
}

fn expr_state(hir: &HirBody, typed: &TypedBody, id: HirExprId) -> ReturnState {
    // Control-flow expressions: use structural analysis and DON'T fall back
    // to the Never-type heuristic. Type inference gives every `loop` type
    // `Never` regardless of whether it contains a reachable `break`, so
    // trusting that here would hide legitimate fall-through cases.
    match &hir.exprs[id] {
        HirExpr::Return { .. } => return ReturnState::Returns,
        HirExpr::Break { .. } | HirExpr::Continue { .. } => return ReturnState::Diverges,

        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let then_s = block_part_state(hir, typed, then_body);
            return match else_body {
                Some(else_block) => then_s.merge(block_part_state(hir, typed, else_block)),
                None => ReturnState::MayFallThrough,
            };
        },

        HirExpr::Match { arms, .. } => {
            // Each arm body sits in tail position of the match expression —
            // a value-producing leaf (Local, Call, Literal) counts as
            // Returns just like the tail of a block does.
            // An empty match is either on a Never type (unreachable) or an
            // error already flagged by the exhaustiveness analyzer (E304);
            // treat it as diverging so we don't emit a cascading E001.
            if arms.is_empty() {
                return ReturnState::Diverges;
            }
            let mut combined = tail_expr_state(hir, typed, arms[0].body);
            for arm in &arms[1..] {
                combined = combined.merge(tail_expr_state(hir, typed, arm.body));
            }
            return combined;
        },

        HirExpr::Loop { body, .. } => {
            // If the body contains a break, the loop may fall through to
            // its successor — even if the body also contains a return on
            // some path, the break can exit before that return runs. This
            // also correctly handles desugared `while`/`for` loops, whose
            // conditional exit is modelled as a `break`.
            return if block_contains_break(hir, body) {
                ReturnState::MayFallThrough
            } else {
                let body_state = block_part_state(hir, typed, body);
                if body_state == ReturnState::Returns {
                    ReturnState::Returns
                } else {
                    ReturnState::Diverges
                }
            };
        },

        HirExpr::Block { body, .. } => return block_part_state(hir, typed, body),

        // Closures don't cause the enclosing function to return
        HirExpr::Closure { .. } => return ReturnState::MayFallThrough,

        _ => {},
    }

    // Leaf-like expressions (calls, literals, field access, ...): if
    // inference proved the expression has type `Never` (e.g. a call to
    // `lang.panic_unwind`), treat it as diverging.
    if matches!(typed.expr_types.get(&id), Some(ResolvedTy::Never)) {
        return ReturnState::Diverges;
    }

    ReturnState::MayFallThrough
}

fn block_part_state(hir: &HirBody, typed: &TypedBody, block: &HirBlock) -> ReturnState {
    block_state(hir, typed, &block.stmts, block.tail_expr)
}

/// Like `expr_state`, but applies the same "value-producing leaf counts
/// as Returns" rule that `block_state` uses for its tail expression.
/// Used for positions that act like tails (match arm bodies).
fn tail_expr_state(hir: &HirBody, typed: &TypedBody, id: HirExprId) -> ReturnState {
    let state = expr_state(hir, typed, id);
    if state.definitely_returns() {
        return state;
    }
    match &hir.exprs[id] {
        HirExpr::If { .. }
        | HirExpr::Match { .. }
        | HirExpr::Loop { .. }
        | HirExpr::Block { .. } => state,
        _ => ReturnState::Returns,
    }
}

// ===== Break detection for loop analysis =====
//
// Checks whether a block contains a `break` that would exit the enclosing loop.
// Does NOT recurse into nested loops (their breaks target the inner loop).

fn block_contains_break(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if stmt_contains_break(hir, stmt_id) {
            return true;
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_contains_break(hir, tail);
    }
    false
}

fn stmt_contains_break(hir: &HirBody, id: HirStmtId) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_contains_break(hir, *expr),
        HirStmt::Let { value: Some(v), .. } => expr_contains_break(hir, *v),
        _ => false,
    }
}

fn expr_contains_break(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Break { .. } => true,

        // Recurse into if/else and match — breaks inside target the outer loop
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_contains_break(hir, then_body)
                || else_body
                    .as_ref()
                    .is_some_and(|e| block_contains_break(hir, e))
        },
        HirExpr::Match { arms, .. } => arms.iter().any(|arm| expr_contains_break(hir, arm.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),

        // Do NOT recurse into nested loops — their breaks target the inner loop
        HirExpr::Loop { .. } => false,

        // Do NOT recurse into closures — their breaks are local
        HirExpr::Closure { .. } => false,

        _ => false,
    }
}
