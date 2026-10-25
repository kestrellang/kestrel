//! # Closure Analyzer
//!
//! Validates closure semantics: implicit `it` parameter arity, closure
//! parameter count and types vs expected function type, and assignment
//! restrictions on captured variables and closure parameters.
//!
//! ## Diagnostics
//!
//! ### E600 — `it_wrong_arity` (Error, Correctness)
//!
//! **Message:** "implicit 'it' parameter used in closure expecting {n} parameters"
//!
//! **Labels:**
//! - Primary: the closure expression
//!   - Span source: `util::expr_span` on the closure `HirExprId`
//!   - Message: "'it' requires exactly 1 parameter"
//!
//! **Notes:** (none)
//!
//! ### E601 — `closure_arity_mismatch` (Error, Correctness)
//!
//! **Message:** "closure has {actual} parameters, but expected {expected}"
//!
//! **Labels:**
//! - Primary: the closure expression
//!   - Span source: `util::expr_span` on the closure `HirExprId`
//!   - Message: "wrong number of parameters"
//!
//! **Notes:** (none)
//!
//! ### E602 — `closure_param_type_mismatch` (Error, Correctness)
//!
//! **Message:** "closure parameter type mismatch at position {index}"
//!
//! **Labels:**
//! - Primary: the closure parameter with wrong type
//!   - Span source: (closure span as fallback)
//!   - Message: "expected '{expected}', got '{actual}'"
//!
//! **Notes:** (none)
//!
//! ### E603 — `assign_to_captured_variable` (Error, Correctness)
//!
//! Fires only for **normal**-kind closures (`closure_kind_of`): a normal
//! closure captures read-only views. `mutating` captures `&mutating` views and
//! `consuming`/`escaping` own their captures, so all three lift the check
//! (docs/design/closures.md, Diagnostics table).
//!
//! **Message:** "cannot assign to captured variable '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the assignment target `HirExprId`
//!   - Message: "captured variables are immutable in closures"
//!
//! **Notes:**
//! - "a normal closure captures read-only views; give it a `mutating` expected
//!   type (e.g. `mutating () -> ()`) to write back to the original, or fold the
//!   value and return it instead"
//!
//! ### E604 — `assign_to_closure_parameter` (Error, Correctness)
//!
//! **Message:** "cannot assign to closure parameter '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the assignment target `HirExprId`
//!   - Message: "closure parameters are immutable"
//!
//! **Notes:** (none)

use std::collections::HashSet;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast::FnTypeKind;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E600",
        name: "it_wrong_arity",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E601",
        name: "closure_arity_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E602",
        name: "closure_param_type_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E603",
        name: "assign_to_captured_variable",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E604",
        name: "assign_to_closure_parameter",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    // NOTE: E605 (`capturing_closure_escape`) used to live here; the check
    // moved to the MIR escape checker (E494, see #174) and the descriptor was
    // deleted. E605 now belongs solely to extern_ffi_safe.rs.
    DiagnosticDescriptor {
        id: "E606",
        name: "cannot_infer_closure_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    // NOTE: E212 (`non_static_capture`) used to live here — "a closure cannot
    // capture a ref binding / a `not Static` value". It is RETIRED
    // (docs/design/closures.md, Diagnostics table: "E212 — retired — view
    // capture is now the default"; plan lockstep 6). A view environment holds
    // ADDRESSES into the frame it was created in and can never outlive it
    // (E494 enforces that), so capturing a reference through one is sound.
    // Only the OWNING tier still rejects frame provenance — that is E624 below.
    //
    // Owning-tier capture rejection (plan D4). An `escaping`/`consuming`
    // environment SNAPSHOTS its captures and may outlive the frame, so a
    // capture that carries frame provenance (a ref binding, a `not Static`
    // value) has no owning representation. Shares E624 with the passing table:
    // both say "this closure kind cannot be formed from this".
    DiagnosticDescriptor {
        id: "E624",
        name: "owning_capture_rejected",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ClosureAnalyzer;

impl Describe for ClosureAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::Closure
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ClosureAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Captures come from the single source of truth — the post-inference
        // ClosureCaptures query (place-based). E603 only needs the set of
        // captured *root* locals.
        let capture_plan = cx.query.query(kestrel_type_infer::ClosureCaptures {
            entity: cx.entity,
            root: cx.root,
        });

        // Walk all expressions looking for closures
        for (expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Closure { params, body, .. } = expr else {
                continue;
            };

            // Only an OWNING env rejects frame provenance (E624). A view env
            // is frame-bound, so it captures ref bindings and `not Static`
            // values like any other place — the retirement of E212.
            let owning_kind = matches!(
                cx.typed.expr_types.get(&expr_id),
                Some(ResolvedTy::Function { kind, .. })
                    if !matches!(kind, kestrel_ast::FnTypeKind::Normal
                        | kestrel_ast::FnTypeKind::Mutating)
            );

            let mut capture_roots: Vec<LocalId> = capture_plan
                .get(expr_id)
                .iter()
                .map(|c| c.key.root)
                .collect();
            capture_roots.sort_by_key(|l| l.raw());
            capture_roots.dedup();
            let captures = &capture_roots;

            // E624 (owning tier only): an owned environment SNAPSHOTS its
            // captures and may outlive this frame, so a capture that carries
            // frame provenance — a ref binding, a `not Static` value — has no
            // owning representation. View kinds skip the whole loop: their env
            // is frame-bound, which is exactly why E212 retired.
            if owning_kind {
                for &root in captures {
                    let Some(ty) = cx.typed.local_types.get(&root) else {
                        continue;
                    };
                    let is_ref = matches!(ty, ResolvedTy::Ref { .. });
                    if !is_ref
                        && crate::staticness::resolved_ty_is_static(
                            cx.query, ty, cx.entity, cx.root,
                        )
                    {
                        continue;
                    }
                    let name = cx.hir.locals[root].name.clone();
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[6].id,
                        severity: DESCRIPTORS[6].default_severity,
                        message: format!(
                            "an owning closure cannot capture '{name}': it carries a reference"
                        ),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, expr_id),
                            message: "captured into an owned environment here".into(),
                            is_primary: true,
                        }],
                        notes: vec![
                            "an owning environment outlives this frame, so it may only own \
                             reference-free (Static) snapshots"
                                .into(),
                            "copy the referenced value into a `let` first, and capture that".into(),
                        ],
                    });
                }
            }

            // Check closure arity and types against expected function type.
            if let Some(ty) = cx.typed.expr_types.get(&expr_id) {
                check_closure_type(cx, expr_id, params, ty, &mut diags);
            }

            // E606: closure param types could not be inferred (no context at all).
            // Only emit when inference produced NO other errors — unresolved params
            // with other errors are likely cascading failures, not missing context.
            if !params.is_empty() && cx.typed.errors.is_empty() {
                let has_unresolved = params.iter().any(|p| {
                    p.ty.is_none()
                        && cx
                            .typed
                            .local_types
                            .get(&p.local)
                            .is_some_and(|t| matches!(t, ResolvedTy::Error))
                });
                if has_unresolved {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[5].id,
                        severity: DESCRIPTORS[5].default_severity,
                        message: "could not infer type for closure parameter".into(),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, expr_id),
                            message: "closure needs type context".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                    continue;
                }
            }

            // E603: check for assignments to captured variables.
            //
            // KIND-GATED (docs/design/closures.md, Diagnostics table): only a
            // NORMAL closure's captures are read-only views. `mutating` makes
            // the views `&mutating` (assignment is the whole point) and
            // `escaping` owns its captures, so both lift the error; `consuming`
            // owns them too. An expected normal type is never silently
            // upgraded — the fix-it note points at the `mutating` spelling.
            if !captures.is_empty() && closure_kind_of(cx, expr_id) == FnTypeKind::Normal {
                let capture_set: HashSet<LocalId> = captures.iter().copied().collect();
                check_capture_assignments(cx, body, &capture_set, &mut diags);
            }

            // NOTE: the capturing-closure escape check (formerly E605, a
            // syntactic "is the literal in return position" test that missed
            // laundering through `let`/aggregates) now lives in the MIR escape
            // checker (`kestrel_mir::verify::check_escapes`, E494). A capturing
            // closure's value is rooted at the join over its captures
            // (`emit_apply_partial`); the same root-provenance rule that rejects
            // returning a `&local` rejects returning a frame-bound closure,
            // through every escape route. Single source of truth (#174).
        }

        diags
    }
}

/// The closure tier inference settled on for this literal. A literal is built
/// at `Normal` and retrofitted in place from the expected type, so this is the
/// kind the closure is actually being *built for*. Absent/non-function type →
/// `Normal` (the default kind).
fn closure_kind_of(cx: &BodyContext<'_>, closure: HirExprId) -> FnTypeKind {
    match cx.typed.expr_types.get(&closure) {
        Some(ResolvedTy::Function { kind, .. }) => *kind,
        _ => FnTypeKind::Normal,
    }
}

/// Check closure parameter count and types against the expected function type.
fn check_closure_type(
    cx: &BodyContext<'_>,
    expr_id: HirExprId,
    params: &[HirClosureParam],
    expected_ty: &ResolvedTy,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let ResolvedTy::Function {
        params: expected_params,
        ..
    } = expected_ty
    else {
        return;
    };

    let actual_count = params.len();
    let expected_count = expected_params.len();

    // NOTE: the implicit-`it` wrong-arity check (E600) deliberately lives in the
    // type-inference solver (`InferError::ItWrongArity`), NOT here. The AST
    // builder injects `it` as an explicit param whenever a closure body uses it,
    // so a real `it`-closure reaches this point with `actual_count == 1`; the
    // solver keys its check on that specific closure literal's TyVar
    // (`closure_it`). A whole-function `locals.iter()…name == "it"` scan here
    // (as a prior version did) false-flagged every zero-param closure that merely
    // had a *sibling* `it`-closure in the same body. Single source of truth: the
    // solver.

    // Arity mismatch (E601)
    if actual_count != expected_count && actual_count > 0 {
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[1].id,
            severity: DESCRIPTORS[1].default_severity,
            message: format!(
                "closure has {} parameters, but expected {}",
                actual_count, expected_count
            ),
            labels: vec![DiagLabel {
                span: util::expr_span(cx.hir, expr_id),
                message: "wrong number of parameters".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
        // Don't check types if counts differ
    }

    // TODO: E602 — per-parameter type mismatch checking.
    // This requires comparing resolved param types against expected_params,
    // which needs the resolved type for each closure param's type annotation.
    // Type mismatch is already caught by the constraint solver, so this is
    // a nice-to-have for better error messages.
}

/// Walk a closure body looking for assignments to closure parameters.
#[allow(dead_code)]
fn check_param_assignments(
    cx: &BodyContext<'_>,
    body: &HirBlock,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &body.stmts {
        check_stmt_for_param_assign(cx, stmt_id, param_locals, diags);
    }
    if let Some(tail) = body.tail_expr {
        check_expr_for_param_assign(cx, tail, param_locals, diags);
    }
}

#[allow(dead_code)]
fn check_stmt_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirStmtId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.stmts[id] {
        HirStmt::Expr { expr, .. } => {
            check_expr_for_param_assign(cx, *expr, param_locals, diags);
        },
        HirStmt::Let { value: Some(v), .. } => {
            check_expr_for_param_assign(cx, *v, param_locals, diags);
        },
        _ => {},
    }
}

#[allow(dead_code)]
fn check_expr_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirExprId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[id] {
        HirExpr::Assign { target, value, .. } => {
            // Check if target is a closure parameter
            if let HirExpr::Local(local_id, _) = &cx.hir.exprs[*target]
                && param_locals.contains(local_id)
            {
                let name = cx.hir.locals[*local_id].name.clone();
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[4].id,
                    severity: DESCRIPTORS[4].default_severity,
                    message: format!("cannot assign to closure parameter '{}'", name),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, *target),
                        message: "closure parameters are immutable".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
            check_expr_for_param_assign(cx, *value, param_locals, diags);
        },

        // Recurse into sub-expressions
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            check_expr_for_param_assign(cx, *condition, param_locals, diags);
            check_block_for_param_assign(cx, then_body, param_locals, diags);
            if let Some(else_block) = else_body {
                check_block_for_param_assign(cx, else_block, param_locals, diags);
            }
        },
        HirExpr::Loop { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        },
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            check_expr_for_param_assign(cx, *scrutinee, param_locals, diags);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    check_expr_for_param_assign(cx, guard, param_locals, diags);
                }
                check_expr_for_param_assign(cx, arm.body, param_locals, diags);
            }
        },
        HirExpr::Block { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        },
        HirExpr::Call { callee, args, .. } => {
            check_expr_for_param_assign(cx, *callee, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            check_expr_for_param_assign(cx, *receiver, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        },
        HirExpr::Return {
            value: Some(val), ..
        } => {
            check_expr_for_param_assign(cx, *val, param_locals, diags);
        },
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            check_expr_for_param_assign(cx, *base, param_locals, diags);
        },
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &elem in elements {
                check_expr_for_param_assign(cx, elem, param_locals, diags);
            }
        },
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                check_expr_for_param_assign(cx, entry.key, param_locals, diags);
                check_expr_for_param_assign(cx, entry.value, param_locals, diags);
            }
        },
        // Don't recurse into nested closures — they have their own param scope
        HirExpr::Closure { .. } => {},

        // Leaf expressions
        _ => {},
    }
}

#[allow(dead_code)]
fn check_block_for_param_assign(
    cx: &BodyContext<'_>,
    block: &HirBlock,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &block.stmts {
        check_stmt_for_param_assign(cx, stmt_id, param_locals, diags);
    }
    if let Some(tail) = block.tail_expr {
        check_expr_for_param_assign(cx, tail, param_locals, diags);
    }
}

/// Walk a closure body looking for assignments to captured variables (E603).
fn check_capture_assignments(
    cx: &BodyContext<'_>,
    block: &HirBlock,
    capture_set: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &block.stmts {
        match &cx.hir.stmts[stmt_id] {
            HirStmt::Expr { expr, .. } => {
                walk_for_capture_assign(cx, *expr, capture_set, diags);
            },
            HirStmt::Let { value: Some(v), .. } => {
                walk_for_capture_assign(cx, *v, capture_set, diags);
            },
            _ => {},
        }
    }
    if let Some(tail) = block.tail_expr {
        walk_for_capture_assign(cx, tail, capture_set, diags);
    }
}

/// The root local of an assignment TARGET place (`c`, `c.n`, `c.n.0`), or
/// `None` when the target is not a place chain (a subscript/getter write, an
/// error node). Deliberately mirrors `place_key_of`'s walk without needing the
/// field resolutions — E603 only cares about the root.
fn assign_target_root(cx: &BodyContext<'_>, target: HirExprId) -> Option<LocalId> {
    match &cx.hir.exprs[target] {
        HirExpr::Local(local, _) => Some(*local),
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            assign_target_root(cx, *base)
        },
        HirExpr::Sugar { inner, .. } => assign_target_root(cx, *inner),
        _ => None,
    }
}

fn walk_for_capture_assign(
    cx: &BodyContext<'_>,
    id: HirExprId,
    capture_set: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[id] {
        HirExpr::Assign { target, value, .. } => {
            // ANY place rooted at a capture, not just the bare local. A view
            // env binds the capture's ADDRESS, so `c.n = 5` writes straight
            // back through the view — and a normal closure's captures are
            // read-only by design (the copy-soundness argument in
            // docs/design/closures.md §"Normal: read-only views" depends on
            // nobody writing through a shared view). Matching only
            // `HirExpr::Local` let every projected write slip past.
            if let Some(local_id) = assign_target_root(cx, *target)
                && capture_set.contains(&local_id)
            {
                let name = cx.hir.locals[local_id].name.clone();
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[3].id,
                    severity: DESCRIPTORS[3].default_severity,
                    message: format!("cannot assign to captured variable '{}'", name),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, *target),
                        message: "captured variables are immutable in closures".into(),
                        is_primary: true,
                    }],
                    // Fix-it (design: "E603 ... with a fix-it suggesting
                    // `mutating`/`escaping`"). A normal closure captures
                    // read-only views; the expected TYPE is what selects a
                    // writable tier — there is no kind-on-literal spelling.
                    notes: vec![
                        "a normal closure captures read-only views; give it a `mutating` \
                         expected type (e.g. `mutating () -> ()`) to write back to the \
                         original, or fold the value and return it instead"
                            .to_string(),
                    ],
                });
            }
            walk_for_capture_assign(cx, *value, capture_set, diags);
        },
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            walk_for_capture_assign(cx, *condition, capture_set, diags);
            check_capture_assignments(cx, then_body, capture_set, diags);
            if let Some(eb) = else_body {
                check_capture_assignments(cx, eb, capture_set, diags);
            }
        },
        HirExpr::Loop { body, .. } | HirExpr::Block { body, .. } => {
            check_capture_assignments(cx, body, capture_set, diags);
        },
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            walk_for_capture_assign(cx, *scrutinee, capture_set, diags);
            for arm in arms {
                if let Some(g) = arm.guard {
                    walk_for_capture_assign(cx, g, capture_set, diags);
                }
                walk_for_capture_assign(cx, arm.body, capture_set, diags);
            }
        },
        HirExpr::Call { callee, args, .. } => {
            walk_for_capture_assign(cx, *callee, capture_set, diags);
            for arg in args {
                walk_for_capture_assign(cx, arg.value, capture_set, diags);
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            walk_for_capture_assign(cx, *receiver, capture_set, diags);
            for arg in args {
                walk_for_capture_assign(cx, arg.value, capture_set, diags);
            }
        },
        HirExpr::Return { value: Some(v), .. } => {
            walk_for_capture_assign(cx, *v, capture_set, diags);
        },
        HirExpr::Closure { .. } => {}, // nested closure has own scope
        _ => {},
    }
}
