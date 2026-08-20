//! # Initializer Verification Analyzer
//!
//! Verifies that struct initializers correctly initialize all stored fields.
//! Tracks which fields of `self` are assigned through control flow, and checks:
//! - All non-computed fields must be assigned before the initializer returns
//! - `let` fields cannot be assigned more than once
//! - Fields cannot be read before they are assigned
//! - `self` cannot be used (e.g., method calls on self) before full initialization
//!
//! Only runs on entities with `NodeKind::Initializer`.
//!
//! ## Diagnostics
//!
//! ### E005 — `uninitialized_fields` (Error, Correctness)
//!
//! **Message:** "initializer does not initialize all fields: {field_list}"
//!
//! **Labels:**
//! - Primary: the closing `}` of the initializer body
//!   - Span source: `util::body_close_brace_span` on the initializer entity
//!     (falls back to `util::entity_span` for bodyless inits)
//!   - Message: "in this initializer"
//!
//! **Notes:** (none)
//!
//! ### E006 — `let_field_assigned_twice` (Error, Correctness)
//!
//! **Message:** "cannot assign to 'let' field '{name}' more than once"
//!
//! **Labels:**
//! - Primary: the second assignment to the let field
//!   - Span source: `util::expr_span` on the assignment target `HirExprId`
//!   - Message: "second assignment here"
//!
//! **Notes:** (none)
//!
//! ### E007 — `field_read_before_assigned` (Error, Correctness)
//!
//! **Message:** "cannot read field '{name}' before it is initialized"
//!
//! **Labels:**
//! - Primary: the field access expression
//!   - Span source: `util::expr_span` on the field read `HirExprId`
//!   - Message: "field read here"
//!
//! **Notes:** (none)
//!
//! ### E008 — `self_used_before_initialized` (Error, Correctness)
//!
//! **Message:** "cannot use 'self' before all fields are initialized"
//!
//! **Labels:**
//! - Primary: the expression using self
//!   - Span source: `util::expr_span` on the `HirExprId`
//!   - Message: "self used here"
//!
//! **Notes:** "uninitialized fields: {list}"
//!
//! ### E009 — `return_before_fully_initialized` (Error, Correctness)
//!
//! **Message:** "cannot return before all fields are initialized"
//!
//! **Labels:**
//! - Primary: the return expression
//!   - Span source: `util::expr_span` on the return `HirExprId`
//!   - Message: "return here"
//!
//! **Notes:** "uninitialized fields: {list}"

use std::collections::HashSet;

use indexmap::IndexSet;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, CstNode, InitEffect, NodeKind, Settable};
use kestrel_hir::body::*;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E005",
        name: "uninitialized_fields",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E006",
        name: "let_field_assigned_twice",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E007",
        name: "field_read_before_assigned",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E008",
        name: "self_used_before_initialized",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E009",
        name: "return_before_fully_initialized",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct InitializerAnalyzer;

impl Describe for InitializerAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::Initializer
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for InitializerAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Only run on initializers
        let kind = cx.query.get::<NodeKind>(cx.entity);
        if !matches!(kind, Some(NodeKind::Initializer)) {
            return vec![];
        }

        // Find the parent struct
        let Some(parent) = cx.query.parent_of(cx.entity) else {
            return vec![];
        };
        if !matches!(cx.query.get::<NodeKind>(parent), Some(NodeKind::Struct)) {
            return vec![];
        }

        // Collect stored fields: children of the struct with NodeKind::Field
        // that have a Gettable component. Computed properties (getters without
        // stored backing) have a Body but we don't distinguish here — all
        // fields with Gettable and no Body are stored. Actually the simplest
        // heuristic: a field child that is Gettable is stored. Computed
        // properties have a Body component.
        // `all_fields` is an `IndexSet`, not a `HashSet`: it is iterated
        // straight into diagnostic *text* — the E005 message and the E008/E009
        // notes below all `join(", ")` it. A `HashSet` made that order random
        // per process (25 runs of one 4-field repro produced 23 distinct
        // orderings), so the same source printed a different message every
        // build. `children_of_kind` yields source-declaration order, so an
        // `IndexSet` preserves it for free — chosen over `BTreeSet` because
        // "the order you wrote the fields in" reads better than alphabetical.
        //
        // `let_fields` stays a `HashSet` deliberately: membership-only, never
        // iterated for display. Same for `InitState::assigned` /
        // `let_assigned`.
        let mut all_fields = IndexSet::new();
        let mut let_fields = HashSet::new();

        for child in util::children_of_kind(cx.query, parent, NodeKind::Field) {
            let name = util::entity_name(cx.query, child);
            // Skip computed properties — they have a Callable component (getter with receiver)
            if cx.query.get::<Callable>(child).is_some() {
                continue;
            }
            // Skip shorthand computed properties (e.g. `var x: T { expr }`) —
            // these have a CodeBlock/PropertyAccessors in the CST but no Callable
            // because the builder doesn't recognize the shorthand form yet.
            if let Some(cst) = cx.query.get::<CstNode>(child) {
                use kestrel_syntax_tree::SyntaxKind;
                let has_body_node = cst.0.children().any(|c| {
                    matches!(
                        c.kind(),
                        SyntaxKind::CodeBlock | SyntaxKind::PropertyAccessors
                    )
                });
                if has_body_node {
                    continue;
                }
            }
            // Skip fields with default values — they don't need explicit init
            if cx.query.get::<kestrel_ast_builder::Body>(child).is_some() {
                continue;
            }
            all_fields.insert(name.clone());
            // A field is `let` if it lacks the Settable marker
            if cx.query.get::<Settable>(child).is_none() {
                let_fields.insert(name);
            }
        }

        if all_fields.is_empty() {
            return vec![];
        }

        let mut vctx = VerifyCtx {
            all_fields: all_fields.clone(),
            let_fields,
            diags: Vec::new(),
            loop_break_stack: Vec::new(),
        };

        let final_state = analyze_block(cx, &cx.hir.statements, cx.hir.tail_expr, &mut vctx);

        // Check that all fields are initialized at the end (unless the body diverges)
        if !final_state.diverged {
            let uninitialized: Vec<&String> = all_fields
                .iter()
                .filter(|f| !final_state.assigned.contains(*f))
                .collect();
            if !uninitialized.is_empty() {
                let field_list = uninitialized
                    .iter()
                    .map(|s| format!("'{}'", s))
                    .collect::<Vec<_>>()
                    .join(", ");
                let span = util::body_close_brace_span(cx.query, cx.entity)
                    .unwrap_or_else(|| util::entity_span(cx.query, cx.entity));
                vctx.diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("initializer does not initialize all fields: {}", field_list),
                    labels: vec![DiagLabel {
                        span,
                        message: "in this initializer".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        vctx.diags
    }
}

// ===== Verification state =====

struct VerifyCtx {
    /// Source-declaration order. Iterated into diagnostic text — see the
    /// comment where it is built.
    all_fields: IndexSet<String>,
    let_fields: HashSet<String>,
    diags: Vec<AnalyzeDiagnostic>,
    /// Stack of per-loop break-state collectors, each keyed by its loop's
    /// label. When a `break` is reached, its current state is pushed onto the
    /// frame of the loop that break actually exits — resolved innermost-first
    /// by `kestrel_hir::label_selects_loop`, exactly as MIR lowering resolves
    /// it. The merge of a frame becomes the outer state after that loop exits.
    ///
    /// This stack is reachability-aware (it only sees breaks on live paths),
    /// which is why `initializer` keeps it instead of using the syntactic
    /// `control_flow::block_contains_break_for` predicate the other analyzers
    /// share. It used to push onto `.last_mut()` unconditionally, so a
    /// `break outer` from a nested loop landed in the *inner* frame; the outer
    /// frame then popped empty, was called an infinite loop, set `diverged`,
    /// and the single gate on `diverged` skipped the whole E005 field check
    /// (G10).
    loop_break_stack: Vec<(Option<String>, Vec<InitState>)>,
}

#[derive(Clone, Debug)]
struct InitState {
    /// Fields that have been definitely assigned
    assigned: HashSet<String>,
    /// Let-fields that have been assigned (for double-assign detection)
    let_assigned: HashSet<String>,
    diverged: bool,
}

impl InitState {
    fn new() -> Self {
        Self {
            assigned: HashSet::new(),
            let_assigned: HashSet::new(),
            diverged: false,
        }
    }

    /// Merge two branch states. Assigned = intersection (must be assigned in both).
    /// Let-assigned = union (if assigned in either branch, counts for double-assign).
    fn merge(self, other: InitState) -> InitState {
        if self.diverged && other.diverged {
            InitState {
                assigned: self
                    .assigned
                    .intersection(&other.assigned)
                    .cloned()
                    .collect(),
                let_assigned: self
                    .let_assigned
                    .union(&other.let_assigned)
                    .cloned()
                    .collect(),
                diverged: true,
            }
        } else if self.diverged {
            other
        } else if other.diverged {
            self
        } else {
            InitState {
                assigned: self
                    .assigned
                    .intersection(&other.assigned)
                    .cloned()
                    .collect(),
                let_assigned: self
                    .let_assigned
                    .union(&other.let_assigned)
                    .cloned()
                    .collect(),
                diverged: false,
            }
        }
    }
}

// ===== Analysis functions =====

fn analyze_block(
    cx: &BodyContext<'_>,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    vctx: &mut VerifyCtx,
) -> InitState {
    let mut state = InitState::new();

    for &stmt_id in stmts {
        if state.diverged {
            break;
        }
        state = analyze_stmt(cx, stmt_id, state, vctx);
    }

    if !state.diverged
        && let Some(tail) = tail
    {
        state = analyze_expr(cx, tail, state, false, vctx);
    }

    state
}

fn analyze_stmt(
    cx: &BodyContext<'_>,
    id: HirStmtId,
    mut state: InitState,
    vctx: &mut VerifyCtx,
) -> InitState {
    match &cx.hir.stmts[id] {
        HirStmt::Let { value, .. } => {
            if let Some(val) = value {
                state = analyze_expr(cx, *val, state, false, vctx);
            }
        },
        HirStmt::Expr { expr, .. } => {
            state = analyze_expr(cx, *expr, state, false, vctx);
        },
        HirStmt::Deinit { .. } => {},
    }
    state
}

fn analyze_expr(
    cx: &BodyContext<'_>,
    id: HirExprId,
    mut state: InitState,
    is_assign_target: bool,
    vctx: &mut VerifyCtx,
) -> InitState {
    match &cx.hir.exprs[id] {
        // Field access on self: check if reading before assigned
        HirExpr::Field { base, name, .. } => {
            if is_self_local(cx, *base) {
                let name_str = name.as_str();
                if !is_assign_target
                    && name_str.is_some_and(|s| vctx.all_fields.contains(s))
                    && name_str.is_some_and(|s| !state.assigned.contains(s))
                {
                    vctx.diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[2].id,
                        severity: DESCRIPTORS[2].default_severity,
                        message: format!("cannot read field '{}' before it is initialized", name),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, id),
                            message: "field read here".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            } else {
                state = analyze_expr(cx, *base, state, false, vctx);
            }
        },

        // `&inner` reads the borrowed place.
        HirExpr::Borrow { inner, .. } => {
            state = analyze_expr(cx, *inner, state, false, vctx);
        },

        // Assignment: check for self.field assignments
        HirExpr::Assign { target, value, .. } => {
            state = analyze_expr(cx, *value, state, false, vctx);

            // Target may be `self.x` (Field) or bare `x` resolved via name lookup
            // (Def pointing at a field of the enclosing struct). Both initialize
            // the field.
            let assigned_field = match &cx.hir.exprs[*target] {
                HirExpr::Field { base, name, .. } if is_self_local(cx, *base) => {
                    name.as_str().map(|s| s.to_string())
                },
                HirExpr::Def(entity, _, _)
                    if cx.query.get::<NodeKind>(*entity) == Some(&NodeKind::Field)
                        && cx.query.parent_of(*entity) == cx.query.parent_of(cx.entity) =>
                {
                    Some(util::entity_name(cx.query, *entity))
                },
                _ => None,
            };

            if let Some(name) = assigned_field
                && vctx.all_fields.contains(&name)
            {
                // Check double-assign to let field
                if vctx.let_fields.contains(&name) && state.let_assigned.contains(&name) {
                    vctx.diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!("cannot assign to 'let' field '{}' more than once", name),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, *target),
                            message: "second assignment here".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
                state.assigned.insert(name.clone());
                if vctx.let_fields.contains(&name) {
                    state.let_assigned.insert(name);
                }
            }

            state = analyze_expr(cx, *target, state, true, vctx);
        },

        // Method call on self: check all fields are initialized first.
        // Special case: self.init(...) is a delegating init call that
        // initializes all fields — mark everything as assigned.
        HirExpr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            if is_self_local(cx, *receiver) {
                if method.as_str() == Some("init") {
                    // Delegating init — all fields are initialized by the delegate
                    for arg in args {
                        state = analyze_expr(cx, arg.value, state, false, vctx);
                    }
                    for field in &vctx.all_fields {
                        state.assigned.insert(field.clone());
                    }
                } else {
                    let uninitialized: Vec<String> = vctx
                        .all_fields
                        .iter()
                        .filter(|f| !state.assigned.contains(*f))
                        .cloned()
                        .collect();
                    if !uninitialized.is_empty() {
                        vctx.diags.push(AnalyzeDiagnostic {
                            descriptor_id: DESCRIPTORS[3].id,
                            severity: DESCRIPTORS[3].default_severity,
                            message: "cannot use 'self' before all fields are initialized".into(),
                            labels: vec![DiagLabel {
                                span: util::expr_span(cx.hir, id),
                                message: "self used here".into(),
                                is_primary: true,
                            }],
                            notes: vec![format!(
                                "uninitialized fields: {}",
                                uninitialized.join(", ")
                            )],
                        });
                    }
                    state = analyze_expr(cx, *receiver, state, false, vctx);
                    for arg in args {
                        state = analyze_expr(cx, arg.value, state, false, vctx);
                    }
                }
            } else {
                state = analyze_expr(cx, *receiver, state, false, vctx);
                for arg in args {
                    state = analyze_expr(cx, arg.value, state, false, vctx);
                }
            }
        },

        // Return: check all fields initialized (skip for failure returns in effectful inits)
        HirExpr::Return { value, .. } => {
            if let Some(val) = value {
                state = analyze_expr(cx, *val, state, false, vctx);
            }
            let is_failure_return = cx.query.get::<InitEffect>(cx.entity).is_some()
                && value.is_some_and(|v| is_init_failure_value(cx, v));
            if !is_failure_return {
                let uninitialized: Vec<String> = vctx
                    .all_fields
                    .iter()
                    .filter(|f| !state.assigned.contains(*f))
                    .cloned()
                    .collect();
                if !uninitialized.is_empty() {
                    vctx.diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[4].id,
                        severity: DESCRIPTORS[4].default_severity,
                        message: "cannot return before all fields are initialized".into(),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, id),
                            message: "return here".into(),
                            is_primary: true,
                        }],
                        notes: vec![format!(
                            "uninitialized fields: {}",
                            uninitialized.join(", ")
                        )],
                    });
                }
            }
            // diverged is set by the Never-type check at the end of analyze_expr
        },

        // If/else: merge branches
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            state = analyze_expr(cx, *condition, state, false, vctx);
            let pre = state.clone();

            let mut then_state = pre.clone();
            for &stmt_id in &then_body.stmts {
                if then_state.diverged {
                    break;
                }
                then_state = analyze_stmt(cx, stmt_id, then_state, vctx);
            }
            if !then_state.diverged
                && let Some(tail) = then_body.tail_expr
            {
                then_state = analyze_expr(cx, tail, then_state, false, vctx);
            }

            let else_state = if let Some(else_block) = else_body {
                let mut es = pre.clone();
                for &stmt_id in &else_block.stmts {
                    if es.diverged {
                        break;
                    }
                    es = analyze_stmt(cx, stmt_id, es, vctx);
                }
                if !es.diverged
                    && let Some(tail) = else_block.tail_expr
                {
                    es = analyze_expr(cx, tail, es, false, vctx);
                }
                es
            } else {
                pre
            };

            state = then_state.merge(else_state);
        },

        // Match: merge all arms
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            state = analyze_expr(cx, *scrutinee, state, false, vctx);
            if !arms.is_empty() {
                let mut iter = arms.iter().map(|arm| {
                    let mut arm_state = state.clone();
                    if let Some(guard) = arm.guard {
                        arm_state = analyze_expr(cx, guard, arm_state, false, vctx);
                    }
                    analyze_expr(cx, arm.body, arm_state, false, vctx)
                });
                let mut merged = iter.next().unwrap();
                for arm_state in iter {
                    merged = merged.merge(arm_state);
                }
                state = merged;
            }
        },

        // Loop: walk the body once; capture the state at every `break` that
        // exits this loop. The post-loop state is the merge of those break
        // states — a field is initialized after the loop only if it was
        // initialized on every path that exits via `break`. A loop with no
        // `break` is an infinite loop and therefore diverges.
        //
        // The early `return` is what keeps this verdict final: the unified
        // Never-type check at the bottom of `analyze_expr` must not get a
        // chance to overrule it. (The old comment justified the return with
        // "all loops have HIR type `Never`" — that is false. `generate.rs`
        // unifies a loop's `break_tv` with unit at every `break` that targets
        // it, so `loop { break; }` is unit-typed. The return is right anyway:
        // structure decides, inference does not. See `docs/fragility/G12/`.)
        //
        // `break_states.is_empty()` is this analyzer's reachability-aware
        // spelling of `!block_contains_break_for` — strictly stronger, since a
        // break on an unreachable path never pushes a state. That is why this
        // one keeps its own walk instead of calling `control_flow`.
        HirExpr::Loop { label, body, .. } => {
            vctx.loop_break_stack.push((label.clone(), Vec::new()));

            let mut body_state = state.clone();
            for &stmt_id in &body.stmts {
                if body_state.diverged {
                    break;
                }
                body_state = analyze_stmt(cx, stmt_id, body_state, vctx);
            }
            if !body_state.diverged
                && let Some(tail) = body.tail_expr
            {
                let _ = analyze_expr(cx, tail, body_state, false, vctx);
            }

            let (_, break_states) = vctx.loop_break_stack.pop().unwrap();
            if break_states.is_empty() {
                // No reachable break → infinite loop
                state.diverged = true;
            } else {
                // Merge all break exit states into the post-loop state
                let mut merged = break_states.into_iter().reduce(|a, b| a.merge(b)).unwrap();
                // A merged set of break exits is not itself diverged — the
                // loop exits normally via the break. `merge` may have set
                // diverged if all breaks came from diverging paths, but the
                // breaks themselves define the exit, so clear it.
                merged.diverged = false;
                state = merged;
            }
            return state;
        },

        // Break: record current state for the enclosing loop's exit merge.
        // Divergence flag is set by the Never-type check below.
        HirExpr::Break { label, .. } => {
            // Attribute the break to the loop it actually exits, not merely
            // the innermost one (G10).
            let frame = vctx
                .loop_break_stack
                .iter_mut()
                .rev()
                .find(|(loop_label, _)| {
                    kestrel_hir::label_selects_loop(label.as_deref(), loop_label.as_deref())
                });
            if let Some((_, break_states)) = frame {
                break_states.push(state.clone());
            }
        },

        // Continue: divergence set by Never-type check; no exit state to record
        HirExpr::Continue { .. } => {},

        // Block expression
        HirExpr::Block { body, .. } => {
            for &stmt_id in &body.stmts {
                if state.diverged {
                    break;
                }
                state = analyze_stmt(cx, stmt_id, state, vctx);
            }
            if !state.diverged
                && let Some(tail) = body.tail_expr
            {
                state = analyze_expr(cx, tail, state, false, vctx);
            }
        },

        // Closures: analyze body separately, don't affect init state
        HirExpr::Closure { body, .. } => {
            let closure_state = state.clone();
            for &stmt_id in &body.stmts {
                let _ = analyze_stmt(cx, stmt_id, closure_state.clone(), vctx);
            }
        },

        // Recurse into other sub-expressions
        HirExpr::Call { callee, args, .. } => {
            state = analyze_expr(cx, *callee, state, false, vctx);
            for arg in args {
                state = analyze_expr(cx, arg.value, state, false, vctx);
            }
        },
        HirExpr::ProtocolCall { receiver, args, .. } => {
            state = analyze_expr(cx, *receiver, state, false, vctx);
            for arg in args {
                state = analyze_expr(cx, arg.value, state, false, vctx);
            }
        },
        HirExpr::TupleIndex { base, .. } => {
            state = analyze_expr(cx, *base, state, false, vctx);
        },
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &elem in elements {
                state = analyze_expr(cx, elem, state, false, vctx);
            }
        },
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                state = analyze_expr(cx, entry.key, state, false, vctx);
                state = analyze_expr(cx, entry.value, state, false, vctx);
            }
        },
        HirExpr::ImplicitMember { args, .. } => {
            if let Some(args) = args {
                for arg in args {
                    state = analyze_expr(cx, arg.value, state, false, vctx);
                }
            }
        },

        // Leaf expressions
        HirExpr::Local(..)
        | HirExpr::Literal { .. }
        | HirExpr::Def(..)
        | HirExpr::OverloadSet { .. }
        | HirExpr::Error { .. } => {},

        // Sugar wrapper: analyze the inner desugared expression transparently.
        HirExpr::Sugar { inner, .. } => {
            state = analyze_expr(cx, *inner, state, is_assign_target, vctx);
        },
    }

    // Unified divergence detection: any expression with Never type diverges.
    // Covers return, break, continue, lang.panic(), and any user function → Never.
    if let Some(ResolvedTy::Never) = cx.typed.expr_types.get(&id) {
        state.diverged = true;
    }

    state
}

/// Check if an expression is a reference to `self` (local named "self").
fn is_self_local(cx: &BodyContext<'_>, expr_id: HirExprId) -> bool {
    if let HirExpr::Local(local_id, _) = &cx.hir.exprs[expr_id] {
        cx.hir.locals[*local_id].name == "self"
    } else {
        false
    }
}

/// Check if a return value represents an init failure path (null literal or .Err(...)).
/// These are allowed to return before all fields are initialized in effectful inits.
fn is_init_failure_value(cx: &BodyContext<'_>, expr_id: HirExprId) -> bool {
    match &cx.hir.exprs[expr_id] {
        HirExpr::Literal {
            value: HirLiteral::Null,
            ..
        } => true,
        HirExpr::ImplicitMember { name, .. } if name.as_str() == Some("Err") => true,
        HirExpr::ImplicitMember { name, .. } if name.as_str() == Some("None") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::BodyCheck;
    use kestrel_ast_builder::{Name, build_declarations, seed_lang_module};
    use kestrel_hecs::{Entity, World};
    use kestrel_hir_lower::LowerBody;
    use kestrel_type_infer::InferBody;

    /// The four field names used by every test here. Deliberately NOT in
    /// alphabetical order, so declaration order is distinguishable from a
    /// `BTreeSet`'s ordering as well as from a `HashSet`'s.
    const FIELDS: &str = "var width: lang.i64; var height: lang.i64; \
                          var alpha: lang.i64; var depth: lang.i64;";
    const EXPECTED: &str = "width, height, alpha, depth";

    /// Parse `source`, run the initializer analyzer over every `init` in it, and
    /// return the diagnostics.
    fn analyze_inits(source: &str) -> Vec<AnalyzeDiagnostic> {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.to_string()));
        seed_lang_module(&mut world, root);

        let file_entity = world.spawn();
        let tokens: Vec<_> = kestrel_lexer::lex(source, file_entity.index())
            .filter_map(|r| r.ok())
            .collect();
        let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
        let result = kestrel_parser::parse_source_file_from_source(source, token_iter);
        build_declarations(&mut world, file_entity, &result.tree, root, None);

        let inits: Vec<Entity> = world
            .iter_component::<NodeKind>()
            .filter_map(|(e, k)| (*k == NodeKind::Initializer).then_some(e))
            .collect();
        assert!(!inits.is_empty(), "test source declares no initializer");

        let ctx = world.query_context();
        let mut diags = Vec::new();
        for entity in inits {
            let Some(hir) = ctx.query(LowerBody { entity, root }) else {
                continue;
            };
            let Some(typed) = ctx.query(InferBody { entity, root }) else {
                continue;
            };
            diags.extend(InitializerAnalyzer.check(&BodyContext {
                query: &ctx,
                entity,
                root,
                hir: &hir,
                typed: &typed,
            }));
        }
        diags
    }

    fn only<'a>(diags: &'a [AnalyzeDiagnostic], code: &str) -> &'a AnalyzeDiagnostic {
        let matching: Vec<_> = diags.iter().filter(|d| d.descriptor_id == code).collect();
        assert_eq!(
            matching.len(),
            1,
            "expected exactly one {code}, got {:?}",
            diags
                .iter()
                .map(|d| (d.descriptor_id, &d.message))
                .collect::<Vec<_>>()
        );
        matching[0]
    }

    /// E005's message lists the uninitialized fields in declaration order.
    ///
    /// Covered by a testdata fixture too; kept here because it is the same
    /// `all_fields` iteration the two note-path tests below exercise, and this
    /// is the one that fails fastest if `all_fields` regresses to a `HashSet`.
    #[test]
    fn e005_message_lists_fields_in_declaration_order() {
        let diags = analyze_inits(&format!(
            "module Test\nstruct Config {{ {FIELDS} init() {{ }} }}"
        ));
        assert_eq!(
            only(&diags, "E005").message,
            "initializer does not initialize all fields: \
             'width', 'height', 'alpha', 'depth'"
        );
    }

    /// E008's *note* lists the uninitialized fields, and the test harness's
    /// diagnostic matcher does not compare notes at all — it pairs annotations
    /// by `(line, message-substring)` only. So this path can only be pinned
    /// here, not by a `.ks` fixture.
    #[test]
    fn e008_note_lists_uninitialized_fields_in_declaration_order() {
        let diags = analyze_inits(&format!(
            "module Test\nstruct Config {{ {FIELDS} \
             func helper() -> () {{ }} \
             init() {{ self.helper(); {} }} }}",
            "self.width = 0; self.height = 0; self.alpha = 0; self.depth = 0;"
        ));
        let e008 = only(&diags, "E008");
        assert_eq!(
            e008.notes,
            vec![format!("uninitialized fields: {EXPECTED}")],
            "E008's note must list fields in declaration order"
        );
    }

    /// Same for E009's note. See the comment on the E008 test for why this
    /// isn't a testdata fixture.
    #[test]
    fn e009_note_lists_uninitialized_fields_in_declaration_order() {
        let diags = analyze_inits(&format!(
            "module Test\nstruct Config {{ {FIELDS} init() {{ return; }} }}"
        ));
        let e009 = only(&diags, "E009");
        assert_eq!(
            e009.notes,
            vec![format!("uninitialized fields: {EXPECTED}")],
            "E009's note must list fields in declaration order"
        );
    }

    /// Repeated in-process runs produce identical text. An `IndexSet` makes
    /// this trivially true; a `HashSet` made it false 16 times out of 25 (the
    /// F43c measurement), and `RandomState` is re-seeded per set, so the
    /// variation shows up within a single process.
    #[test]
    fn field_ordering_is_stable_across_repeated_analyses() {
        let source = format!("module Test\nstruct Config {{ {FIELDS} init() {{ }} }}");
        let first = only(&analyze_inits(&source), "E005").message.clone();
        for _ in 0..50 {
            assert_eq!(
                only(&analyze_inits(&source), "E005").message,
                first,
                "E005 field order varies between runs"
            );
        }
    }
}
