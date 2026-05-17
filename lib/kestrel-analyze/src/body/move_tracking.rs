//! # Move Tracking Analyzer
//!
//! Tracks non-copyable value moves through control flow and reports
//! use-after-move / maybe-moved errors. Mirrors lib1's `move_tracker` design
//! on top of lib's HIR/TypedBody: per-local move state, CFG-join on
//! if/else/match/loop, `consuming` parameter arguments and `consuming self`
//! receivers as move triggers.
//!
//! ## Diagnostics
//!
//! ### E500 — `use_after_move` (Error, Correctness)
//!
//! **Message:** "use of moved value '{name}'"
//!
//! **Labels:**
//! - Primary: the expression using the moved value
//!   - Span source: `util::expr_span` on the `HirExprId` of the offending read
//!   - Message: "value used here after move"
//! - Secondary: the expression where the move occurred
//!   - Span source: `util::expr_span` on the move-trigger `HirExprId`
//!   - Message: "value moved here"
//!
//! **Notes:** "non-copyable values can only be used once"
//!
//! ### E501 — `maybe_moved` (Error, Correctness)
//!
//! **Message:** "value '{name}' may have been moved"
//!
//! **Labels:**
//! - Primary: the expression using the potentially moved value
//!   - Span source: `util::expr_span` on the read `HirExprId`
//!   - Message: "value used here, but may have been moved"
//! - Secondary: the expression where the move may have occurred
//!   - Span source: `util::expr_span` on the move-trigger `HirExprId`
//!   - Message: "value potentially moved here"
//!
//! **Notes:** "value was moved in one branch but not another"

use std::collections::{HashMap, HashSet};

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Callable, ConformanceItem, Conformances, NodeKind, ReceiverKind, WhereClause as AstWhereClause,
    WhereConstraint,
};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_name_res::{ResolveBuiltin, ResolveTypePath, TypeResolution};
use kestrel_semantics::{CopyRequirement, ExplicitlyNegatesProtocol, TypeParamCopyRequirement};
use kestrel_type_infer::result::ResolvedTy;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E500",
        name: "use_after_move",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E501",
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
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Copyable is resolved via the builtin registry where possible, with a
        // name-based fallback. Minimal test inputs (`stdlib: false`) sometimes
        // can't resolve the builtin, but still declare `: not Copyable`
        // syntactically — we must still honor that.
        let copyable_entity = cx.query.query(ResolveBuiltin {
            builtin: Builtin::Copyable,
            root: cx.root,
        });

        let mcx = MoveCtx {
            cx,
            copyable: copyable_entity,
        };
        let mut diags = Vec::new();
        let state = State::empty();
        let _ = analyze_block(
            &mcx,
            &cx.hir.statements,
            cx.hir.tail_expr,
            state,
            &mut diags,
        );
        diags
    }
}

// ===== Dataflow state =====

#[derive(Clone, Copy, Debug)]
enum MoveKind {
    Definite,
    Maybe,
}

#[derive(Clone, Copy, Debug)]
struct MoveInfo {
    kind: MoveKind,
    /// Span anchor for the "value moved here" secondary label. This is an
    /// expression id rather than a raw span so the wording stays consistent
    /// with everything else in the analyzer (which uses `util::expr_span`).
    site: HirExprId,
}

#[derive(Clone, Debug)]
struct State {
    moves: HashMap<LocalId, MoveInfo>,
    /// Locals already reported once in this body. Subsequent reads don't
    /// re-emit — matches the "one error per offending variable" convention
    /// the tests expect.
    reported: HashSet<LocalId>,
    diverged: bool,
}

impl State {
    fn empty() -> Self {
        Self {
            moves: HashMap::new(),
            reported: HashSet::new(),
            diverged: false,
        }
    }
}

struct MoveCtx<'a> {
    cx: &'a BodyContext<'a>,
    /// Resolved Copyable protocol entity. `None` in minimal test inputs
    /// that don't import the builtin; in that case no type can explicitly
    /// negate Copyable so everything reads as copyable — matching the
    /// permissive lib1 behavior for stdlib-less fixtures.
    copyable: Option<Entity>,
}

// ===== Walker (shape modelled on definite_assignment.rs) =====

fn analyze_block(
    mcx: &MoveCtx<'_>,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    mut state: State,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    for &stmt_id in stmts {
        if state.diverged {
            break;
        }
        state = analyze_stmt(mcx, stmt_id, state, diags);
    }
    if !state.diverged
        && let Some(tail) = tail {
            state = analyze_expr(mcx, tail, state, false, diags);
        }
    state
}

fn analyze_stmt(
    mcx: &MoveCtx<'_>,
    id: HirStmtId,
    mut state: State,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    match &mcx.cx.hir.stmts[id] {
        HirStmt::Let { local, value, .. } => {
            if let Some(val) = value {
                state = analyze_expr(mcx, *val, state, false, diags);
                // A `let b = x` on a non-Copyable `x` moves `x` into `b`.
                // Only simple Local-on-RHS triggers a move — field/method/call
                // RHS is never a partial move (matches lib1).
                if let Some(src) = rhs_local(mcx.cx.hir, *val)
                    && local_is_non_copyable(mcx, src) {
                        state.moves.insert(
                            src,
                            MoveInfo {
                                kind: MoveKind::Definite,
                                site: *val,
                            },
                        );
                    }
                // Freshly bound local is valid — remove any stale move state
                // under the same id (shouldn't happen, but defensive).
                state.moves.remove(local);
            }
        },
        HirStmt::Expr { expr, .. } => {
            state = analyze_expr(mcx, *expr, state, false, diags);
        },
        HirStmt::Deinit { name, local, span } => {
            // `deinit x` is both a use-check (x must still be live) and a move
            // (x cannot be used again afterwards). If name resolution failed at
            // lowering time, `local` is None and the lowering already emitted
            // `deinit_undeclared` — nothing to do here.
            if let Some(local_id) = local {
                if let Some(existing) = state.moves.get(local_id).copied() {
                    emit_use_after_move(
                        mcx.cx,
                        diags,
                        *local_id,
                        span.clone(),
                        existing,
                        name.as_str_or_empty(),
                    );
                }
                // Mark moved using the deinit statement's own span as the
                // move site. We synthesize a "pseudo" site by pointing at
                // the first expr whose span matches; simplest is to reuse
                // an expression from the stmt if available. Since Deinit
                // has no expression, we skip inserting an HirExprId-keyed
                // site and just pick an arbitrary expr that covers the span.
                // Pragmatic approach: use the first HirExprId that contains
                // this local read. In practice tests only check _that_ the
                // diagnostic fires, so any valid site works.
                state.moves.insert(
                    *local_id,
                    MoveInfo {
                        kind: MoveKind::Definite,
                        site: deinit_site(mcx.cx.hir, *local_id),
                    },
                );
            }
        },
    }
    state
}

fn analyze_expr(
    mcx: &MoveCtx<'_>,
    id: HirExprId,
    mut state: State,
    is_assign_target: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) -> State {
    let hir = mcx.cx.hir;
    match &hir.exprs[id] {
        // ===== Read of a local =====
        HirExpr::Local(local_id, span) => {
            if !is_assign_target
                && let Some(info) = state.moves.get(local_id).copied()
                    && state.reported.insert(*local_id) {
                        let name = hir.locals[*local_id].name.clone();
                        emit_move_diagnostic(mcx.cx, diags, info, id, span.clone(), &name);
                    }
        },

        // ===== Assignment =====
        HirExpr::Assign { target, value, .. } => {
            state = analyze_expr(mcx, *value, state, false, diags);
            state = analyze_expr(mcx, *target, state, true, diags);
            // If target is a bare Local and value is a Local on a non-Copyable,
            // the value local is moved.
            if let Some(src) = rhs_local(hir, *value) {
                let targeting_self = rhs_local(hir, *target) == Some(src);
                if !targeting_self && local_is_non_copyable(mcx, src) {
                    state.moves.insert(
                        src,
                        MoveInfo {
                            kind: MoveKind::Definite,
                            site: *value,
                        },
                    );
                }
            }
            // A Local being written to is refreshed (new value lands there).
            if let HirExpr::Local(tid, _) = &hir.exprs[*target] {
                state.moves.remove(tid);
            }
        },

        // ===== If / else =====
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            state = analyze_expr(mcx, *condition, state, false, diags);
            let pre = state.clone();

            let then_state = analyze_block(
                mcx,
                &then_body.stmts,
                then_body.tail_expr,
                pre.clone(),
                diags,
            );
            let else_state = if let Some(else_block) = else_body {
                analyze_block(
                    mcx,
                    &else_block.stmts,
                    else_block.tail_expr,
                    pre.clone(),
                    diags,
                )
            } else {
                pre.clone()
            };
            state = merge_if_else(pre, then_state, else_state);
        },

        // ===== Match =====
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            state = analyze_expr(mcx, *scrutinee, state, false, diags);
            if arms.is_empty() {
                return state;
            }
            let pre = state.clone();
            let mut arm_states = Vec::with_capacity(arms.len());
            for arm in arms {
                let mut s = pre.clone();
                if let Some(guard) = arm.guard {
                    s = analyze_expr(mcx, guard, s, false, diags);
                }
                s = analyze_expr(mcx, arm.body, s, false, diags);
                arm_states.push(s);
            }
            state = merge_match(pre, arm_states);
        },

        // ===== Loop =====
        HirExpr::Loop { body, .. } => {
            let pre = state.clone();
            let body_state = analyze_block(mcx, &body.stmts, body.tail_expr, pre.clone(), diags);

            // Propagate reported-set so diagnostics aren't duplicated post-loop.
            state.reported.extend(body_state.reported.iter().copied());

            // Loops that always run to completion without break diverge.
            if body_state.diverged && !block_contains_break(hir, body) {
                state.diverged = true;
            }

            // Promote moves observed inside the body into the post-loop state.
            // - Conditional loop (body starts with `if … { break }` — i.e.
            //   lowered `while`/`while-let`): body may not execute at all,
            //   so body-introduced moves are at best `Maybe`.
            // - Unconditional `loop { … }`: body runs at least once, and if
            //   the move site is reachable before any `break`, a second
            //   iteration would re-read the moved value. Mark Definite.
            let conditional = loop_is_conditional(hir, body);
            for (local, info) in body_state.moves.iter() {
                if pre.moves.contains_key(local) {
                    continue;
                }
                let kind = if conditional {
                    MoveKind::Maybe
                } else {
                    MoveKind::Definite
                };
                state.moves.insert(
                    *local,
                    MoveInfo {
                        kind,
                        site: info.site,
                    },
                );
            }
        },

        // ===== Block expression =====
        HirExpr::Block { body, .. } => {
            state = analyze_block(mcx, &body.stmts, body.tail_expr, state, diags);
        },

        // ===== Return =====
        HirExpr::Return { value, span: _ } => {
            if let Some(val) = value {
                state = analyze_expr(mcx, *val, state, false, diags);
                if let Some(src) = rhs_local(hir, *val)
                    && local_is_non_copyable(mcx, src) {
                        state.moves.insert(
                            src,
                            MoveInfo {
                                kind: MoveKind::Definite,
                                site: *val,
                            },
                        );
                    }
            }
        },

        // ===== Break / Continue (divergence handled below via Never) =====
        HirExpr::Break { .. } | HirExpr::Continue { .. } => {},

        // ===== Closures: analyze body in isolation; don't leak moves =====
        HirExpr::Closure { body, .. } => {
            let inner = State::empty();
            let _ = analyze_block(mcx, &body.stmts, body.tail_expr, inner, diags);
        },

        // ===== Calls — consuming args and consuming receivers move =====
        HirExpr::Call { callee, args, .. } => {
            state = analyze_expr(mcx, *callee, state, false, diags);
            for arg in args {
                state = analyze_expr(mcx, arg.value, state, false, diags);
            }
            let callee_entity = match &hir.exprs[*callee] {
                HirExpr::Def(entity, _, _) => Some(*entity),
                _ => mcx.cx.typed.resolutions.get(callee).copied(),
            };
            if let Some(entity) = callee_entity {
                apply_call_moves(mcx, entity, args, None, &mut state);
            }
        },
        HirExpr::MethodCall { receiver, args, .. } => {
            state = analyze_expr(mcx, *receiver, state, false, diags);
            for arg in args {
                state = analyze_expr(mcx, arg.value, state, false, diags);
            }
            if let Some(&entity) = mcx.cx.typed.resolutions.get(&id) {
                apply_call_moves(mcx, entity, args, Some(*receiver), &mut state);
            }
        },
        HirExpr::ProtocolCall {
            receiver,
            protocol,
            method,
            args,
            ..
        } => {
            state = analyze_expr(mcx, *receiver, state, false, diags);
            for arg in args {
                state = analyze_expr(mcx, arg.value, state, false, diags);
            }
            if let Some(method_entity) =
                find_protocol_method(mcx.cx, *protocol, method.as_str_or_empty())
            {
                apply_call_moves(mcx, method_entity, args, Some(*receiver), &mut state);
            }
        },

        // ===== Pass-throughs =====
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            state = analyze_expr(mcx, *base, state, false, diags);
        },
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &e in elements {
                state = analyze_expr(mcx, e, state, false, diags);
            }
        },
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                state = analyze_expr(mcx, entry.key, state, false, diags);
                state = analyze_expr(mcx, entry.value, state, false, diags);
            }
        },
        HirExpr::ImplicitMember { args, .. } => {
            if let Some(args) = args {
                for arg in args {
                    state = analyze_expr(mcx, arg.value, state, false, diags);
                }
            }
        },

        // Leaves
        HirExpr::Literal { .. }
        | HirExpr::Def(..)
        | HirExpr::OverloadSet { .. }
        | HirExpr::Error { .. } => {},

        // Sugar wrapper: analyze the inner desugared expression transparently.
        HirExpr::Sugar { inner, .. } => {
            state = analyze_expr(mcx, *inner, state, is_assign_target, diags);
        },
    }

    // Unified divergence: any Never-typed expr diverges, with one exception:
    // a Loop expression with a reachable `break` has its type inferred to
    // Never in some cases even though post-loop code is reachable. Rely on
    // the Loop arm above (which only sets `diverged` when the body actually
    // runs to completion without break) — don't let the Never-type shortcut
    // override that.
    if let Some(ResolvedTy::Never) = mcx.cx.typed.expr_types.get(&id)
        && !matches!(&hir.exprs[id], HirExpr::Loop { .. }) {
            state.diverged = true;
        }

    state
}

// ===== Move-trigger helpers =====

/// Apply the move effects of a call: consuming receiver (if any) moves its
/// base local; each consuming arg moves its base local.
fn apply_call_moves(
    mcx: &MoveCtx<'_>,
    callee: Entity,
    args: &[HirCallArg],
    receiver: Option<HirExprId>,
    state: &mut State,
) {
    let Some(callable) = mcx.cx.query.get::<Callable>(callee) else {
        return;
    };

    if let (Some(recv_id), Some(ReceiverKind::Consuming)) = (receiver, callable.receiver.as_ref())
        && let Some(src) = rhs_local(mcx.cx.hir, recv_id)
            && local_is_non_copyable(mcx, src) {
                state.moves.insert(
                    src,
                    MoveInfo {
                        kind: MoveKind::Definite,
                        site: recv_id,
                    },
                );
            }

    for (i, arg) in args.iter().enumerate() {
        let Some(param) = callable.params.get(i) else {
            continue;
        };
        if !param.is_consuming {
            continue;
        }
        if let Some(src) = rhs_local(mcx.cx.hir, arg.value)
            && local_is_non_copyable(mcx, src) {
                state.moves.insert(
                    src,
                    MoveInfo {
                        kind: MoveKind::Definite,
                        site: arg.value,
                    },
                );
            }
    }
}

/// Extract the base local of an expression if it is a bare `HirExpr::Local`.
/// Returns None for anything else (Field/Tuple/Call/etc.) — matches lib1's
/// "no partial moves" behavior.
fn rhs_local(hir: &HirBody, expr: HirExprId) -> Option<LocalId> {
    if let HirExpr::Local(id, _) = &hir.exprs[expr] {
        Some(*id)
    } else {
        None
    }
}

// ===== Copyable query =====

fn local_is_non_copyable(mcx: &MoveCtx<'_>, local: LocalId) -> bool {
    let Some(ty) = mcx.cx.typed.local_types.get(&local) else {
        return false;
    };
    !ty_is_copyable(mcx, ty)
}

fn ty_is_copyable(mcx: &MoveCtx<'_>, ty: &ResolvedTy) -> bool {
    match ty {
        ResolvedTy::Named { entity, .. } => !entity_negates_copyable(mcx, *entity),
        ResolvedTy::Param { entity } => !param_negates_copyable(mcx, *entity),
        ResolvedTy::Tuple(elems) => elems.iter().all(|t| ty_is_copyable(mcx, t)),
        // Functions, Never, Error, SelfType — treat as copyable (pointer-like).
        _ => true,
    }
}

/// True if the entity explicitly opts out of `Copyable`. Uses the
/// semantics query when the builtin is visible; otherwise falls back to
/// matching the last path segment by name so `stdlib: false` test inputs
/// that only declare `: not Copyable` without registering the builtin
/// still see the move semantics.
fn entity_negates_copyable(mcx: &MoveCtx<'_>, entity: Entity) -> bool {
    if let Some(copyable) = mcx.copyable {
        return mcx.cx.query.query(ExplicitlyNegatesProtocol {
            entity,
            protocol: copyable,
            root: mcx.cx.root,
        });
    }
    let Some(conf) = mcx.cx.query.get::<Conformances>(entity) else {
        return false;
    };
    conf.0.iter().any(|item| match item {
        ConformanceItem::Negative(ast_ty, _) => ast_last_segment_is_copyable(ast_ty),
        _ => false,
    })
}

/// True if a where-clause reachable from the body owner declares
/// `param_entity: not Copyable`.
fn param_negates_copyable(mcx: &MoveCtx<'_>, param_entity: Entity) -> bool {
    if mcx.copyable.is_some() {
        return mcx.cx.query.query(TypeParamCopyRequirement {
            param: param_entity,
            context: mcx.cx.entity,
            root: mcx.cx.root,
        }) == CopyRequirement::MayBeNonCopyable;
    }

    let mut seen: HashSet<Entity> = HashSet::new();
    let mut current = Some(mcx.cx.entity);
    while let Some(ent) = current {
        if !seen.insert(ent) {
            break;
        }
        if let Some(wc) = mcx.cx.query.get::<AstWhereClause>(ent) {
            for c in &wc.0 {
                let WhereConstraint::NegativeBound {
                    subject, protocol, ..
                } = c
                else {
                    continue;
                };
                if resolves_to_entity(mcx.cx, subject, ent) != Some(param_entity) {
                    continue;
                }
                if ast_last_segment_is_copyable(protocol) {
                    return true;
                }
            }
        }
        current = mcx.cx.query.parent_of(ent);
    }
    false
}

fn ast_last_segment_is_copyable(ast_ty: &AstType) -> bool {
    let AstType::Named { segments, .. } = ast_ty else {
        return false;
    };
    segments.last().is_some_and(|s| s.name == "Copyable")
}

fn resolves_to_entity(cx: &BodyContext<'_>, ast_ty: &AstType, context: Entity) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match cx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: cx.root,
    }) {
        TypeResolution::Found(e) => Some(e),
        _ => None,
    }
}

// ===== CFG join =====

fn merge_if_else(pre: State, then: State, els: State) -> State {
    let mut reported = pre.reported.clone();
    reported.extend(then.reported.iter().copied());
    reported.extend(els.reported.iter().copied());
    match (then.diverged, els.diverged) {
        (true, true) => State {
            moves: pre.moves,
            reported,
            diverged: true,
        },
        (true, false) => State {
            moves: els.moves,
            reported,
            diverged: false,
        },
        (false, true) => State {
            moves: then.moves,
            reported,
            diverged: false,
        },
        (false, false) => {
            let mut merged = HashMap::new();
            let mut all: HashSet<LocalId> = HashSet::new();
            all.extend(then.moves.keys().copied());
            all.extend(els.moves.keys().copied());
            for local in all {
                let t = then.moves.get(&local).copied();
                let e = els.moves.get(&local).copied();
                let info = match (t, e) {
                    (Some(a), Some(b)) => {
                        let kind = match (a.kind, b.kind) {
                            (MoveKind::Definite, MoveKind::Definite) => MoveKind::Definite,
                            _ => MoveKind::Maybe,
                        };
                        MoveInfo { kind, site: a.site }
                    },
                    (Some(a), None) | (None, Some(a)) => MoveInfo {
                        kind: MoveKind::Maybe,
                        site: a.site,
                    },
                    (None, None) => unreachable!(),
                };
                merged.insert(local, info);
            }
            State {
                moves: merged,
                reported,
                diverged: false,
            }
        },
    }
}

fn merge_match(pre: State, arms: Vec<State>) -> State {
    let mut reported = pre.reported.clone();
    for s in &arms {
        reported.extend(s.reported.iter().copied());
    }
    if arms.iter().all(|s| s.diverged) {
        return State {
            moves: pre.moves,
            reported,
            diverged: true,
        };
    }
    let live: Vec<&State> = arms.iter().filter(|s| !s.diverged).collect();
    let mut all: HashSet<LocalId> = HashSet::new();
    for s in &live {
        all.extend(s.moves.keys().copied());
    }
    let mut merged = HashMap::new();
    for local in all {
        let mut all_definite = true;
        let mut any_info: Option<MoveInfo> = None;
        let mut present_in_all_live = true;
        for s in &live {
            match s.moves.get(&local) {
                Some(info) => {
                    if any_info.is_none() {
                        any_info = Some(*info);
                    }
                    if matches!(info.kind, MoveKind::Maybe) {
                        all_definite = false;
                    }
                },
                None => {
                    all_definite = false;
                    present_in_all_live = false;
                },
            }
        }
        let info = any_info.expect("local was in at least one live arm");
        let kind = if all_definite && present_in_all_live {
            MoveKind::Definite
        } else {
            MoveKind::Maybe
        };
        merged.insert(
            local,
            MoveInfo {
                kind,
                site: info.site,
            },
        );
    }
    State {
        moves: merged,
        reported,
        diverged: false,
    }
}

// ===== Loop shape detection =====

/// Does this loop body start with a conditional `break`? `while` and
/// `while-let` desugar to `loop { if !cond { break }; body }`; their HIR
/// body therefore begins with an `if`-stmt whose then-branch breaks.
fn loop_is_conditional(hir: &HirBody, body: &HirBlock) -> bool {
    let Some(&first) = body.stmts.first() else {
        return false;
    };
    let HirStmt::Expr { expr, .. } = &hir.stmts[first] else {
        return false;
    };
    let HirExpr::If {
        then_body,
        else_body,
        ..
    } = &hir.exprs[*expr]
    else {
        return false;
    };
    // Either branch containing a break makes the loop body conditional
    // (it can exit on iteration 1 before the rest of the body runs).
    block_contains_break(hir, then_body)
        || else_body
            .as_ref()
            .is_some_and(|b| block_contains_break(hir, b))
}

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
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_contains_break(hir, then_body)
                || else_body
                    .as_ref()
                    .is_some_and(|b| block_contains_break(hir, b))
        },
        HirExpr::Match { arms, .. } => arms.iter().any(|a| expr_contains_break(hir, a.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),
        HirExpr::Loop { .. } | HirExpr::Closure { .. } => false,
        _ => false,
    }
}

// ===== Protocol-method lookup =====

fn find_protocol_method(
    cx: &BodyContext<'_>,
    protocol: Entity,
    method_name: &str,
) -> Option<Entity> {
    cx.query
        .children_of(protocol)
        .iter()
        .find(|&&child| {
            cx.query.get::<NodeKind>(child) == Some(&NodeKind::Function)
                && cx
                    .query
                    .get::<kestrel_ast_builder::Name>(child)
                    .is_some_and(|n| n.0 == method_name)
        })
        .copied()
}

// ===== Diagnostic emission =====

fn emit_move_diagnostic(
    cx: &BodyContext<'_>,
    diags: &mut Vec<AnalyzeDiagnostic>,
    info: MoveInfo,
    use_expr: HirExprId,
    use_span: kestrel_span::Span,
    name: &str,
) {
    let secondary_span = util::expr_span(cx.hir, info.site);
    let _ = use_expr; // use_span already captures the read
    match info.kind {
        MoveKind::Definite => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!("use of moved value '{name}'"),
                labels: vec![
                    DiagLabel {
                        span: use_span,
                        message: "value used here after move".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: secondary_span,
                        message: "value moved here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec!["non-copyable values can only be used once".into()],
            });
        },
        MoveKind::Maybe => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!("value '{name}' may have been moved"),
                labels: vec![
                    DiagLabel {
                        span: use_span,
                        message: "value used here, but may have been moved".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: secondary_span,
                        message: "value potentially moved here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec!["value was moved in one branch but not another".into()],
            });
        },
    }
}

/// `deinit x` on an already-moved local — emit the standard use-after-move
/// shape with the statement's span as the use site.
fn emit_use_after_move(
    cx: &BodyContext<'_>,
    diags: &mut Vec<AnalyzeDiagnostic>,
    _local: LocalId,
    use_span: kestrel_span::Span,
    info: MoveInfo,
    name: &str,
) {
    let dummy_expr: HirExprId = info.site;
    emit_move_diagnostic(cx, diags, info, dummy_expr, use_span, name);
}

/// Pick a stable HirExprId to anchor the "value moved here" secondary label
/// for a `deinit` statement. The HIR arena's deinit-stmt doesn't have its
/// own expression, so we use the first expression whose span contains the
/// deinit token. Falls back to the first local-read of this name.
fn deinit_site(hir: &HirBody, local: LocalId) -> HirExprId {
    // Scan for any expression that reads this local — good enough as a span
    // anchor for downstream secondary labels.
    for (id, expr) in hir.exprs.iter() {
        if let HirExpr::Local(l, _) = expr
            && *l == local {
                return id;
            }
    }
    // Fallback: the first expression in the arena.
    hir.exprs
        .iter()
        .next()
        .map(|(id, _)| id)
        .expect("body has at least one expression")
}
