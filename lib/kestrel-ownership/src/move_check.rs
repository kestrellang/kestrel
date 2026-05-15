//! Move-check — emits E500 (use_after_move) and E501 (maybe_moved) by
//! walking each statement's reads against the init/maybe-init dataflow.
//!
//! ## Stage 7 (current)
//!
//! Now the sole emitter of E500/E501; the legacy HIR-level tracker in
//! `kestrel-analyze::body::move_tracking` is being removed. Diagnostic
//! wording is held byte-identical to the HIR version so existing
//! `// ERROR: use of moved value` annotations continue to match.
//!
//! ## Algorithm
//!
//! For each function body:
//!
//! 1. Build the [`MovePathSet`] and run the forward [`dataflow`] to a
//!    fixed point.
//! 2. Walk each block, threading the entry state through statement by
//!    statement. Before applying each statement's transfer function,
//!    look at every place it READS:
//!    - If the underlying path is not `MaybeInit` at this program point,
//!      emit E500 (the path has definitely been moved out on every CFG
//!      route reaching this read).
//!    - If the path is `MaybeInit` but not `DefinitelyInit`, emit E501
//!      (the path was moved on some CFG route but not all).
//! 3. After validating reads, apply the standard transfer function so
//!    the next statement sees the post-kill / post-gen state.
//! 4. Mark each (path, body) as "reported" after the first diagnostic
//!    to avoid duplicate errors when a chain of reads all use the same
//!    moved local — matches the HIR tracker's one-error-per-local rule.
//!
//! Spans are sourced from `Statement.span`, plumbed through MIR lowering
//! via `BodyLowerCtx::current_span`. When a statement has no span (rare —
//! happens for legacy lowering paths that haven't been span-plumbed yet),
//! the diagnostic falls back to a function-body-wide span fetched from
//! the first available statement; the wording still matches but the line
//! may point at the function header instead of the precise read.

use std::collections::HashSet;

use kestrel_mir::{
    BasicBlock, MirBody, MirModule, Place, Rvalue, Statement, StatementKind, TerminatorKind, Value,
};
use kestrel_span::Span;

use crate::Diagnostics;
use crate::dataflow::{self, InitState};
use crate::move_path::{MovePathId, MovePathSet};

/// Move-check diagnostic kinds. Wording is held byte-identical to the
/// legacy `kestrel-analyze::body::move_tracking` analyzer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveDiagKind {
    /// E500 — use of a definitely-moved value.
    UseAfterMove,
    /// E501 — use of a maybe-moved value (moved on some predecessor edges
    /// but not others).
    MaybeMoved,
}

/// One MIR-level move-check diagnostic. Compiler driver / test harness
/// converts these to `kestrel_analyze::AnalyzeDiagnostic`s.
#[derive(Debug, Clone)]
pub struct MoveDiag {
    pub kind: MoveDiagKind,
    /// Local name to interpolate into the "use of moved value 'X'" /
    /// "value 'X' may have been moved" message.
    pub local_name: String,
    /// Primary label site — where the moved value was read.
    pub use_site: Span,
    /// Secondary label site — where the value was moved (best-effort;
    /// `None` when the dataflow didn't observe a span for the kill).
    pub move_site: Option<Span>,
}

/// Diagnostic descriptors for move-check. Mirrors the descriptors the
/// legacy HIR tracker exposed via `kestrel-analyze::body::move_tracking`.
pub const E500_USE_AFTER_MOVE: &str = "E500";
pub const E501_MAYBE_MOVED: &str = "E501";

pub fn run(module: &mut MirModule, diags: &mut Diagnostics) {
    for func in &module.functions {
        let Some(body) = &func.body else { continue };
        // Skip bodies that contain `MirTy::Error` locals — those come from
        // upstream type-inference failures, and running move-check over a
        // broken type structure produces noisy false positives whose
        // diagnostics would overshadow the real type errors. The user
        // already sees the inference diagnostic; the move-check has
        // nothing useful to add until lowering succeeded.
        if body.locals.iter().any(|l| l.ty.contains_error()) {
            continue;
        }
        let paths = MovePathSet::build(body, module, func.where_clause.as_ref());
        if paths.is_empty() {
            continue;
        }
        let result = dataflow::run(body, &paths);
        check_body(body, &paths, &result, diags);
    }
}

fn check_body(
    body: &MirBody,
    paths: &MovePathSet,
    df: &dataflow::DataflowResult,
    diags: &mut Diagnostics,
) {
    // One diagnostic per (path, body) — matches HIR `state.reported` rule.
    let mut reported: HashSet<MovePathId> = HashSet::new();
    for (bi, block) in body.blocks.iter().enumerate() {
        check_block(body, block, bi, paths, df, &mut reported, diags);
    }
}

fn check_block(
    body: &MirBody,
    block: &BasicBlock,
    bi: usize,
    paths: &MovePathSet,
    df: &dataflow::DataflowResult,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    let mut state = df.blocks[bi].entry.clone();
    for stmt in &block.stmts {
        check_statement_reads(body, stmt, &state, paths, reported, diags);
        dataflow::apply_statement(&mut state, stmt, paths);
    }
    let term_span = block.terminator.span.clone();
    check_terminator_reads(
        body,
        &block.terminator.kind,
        term_span.as_ref(),
        &state,
        paths,
        reported,
        diags,
    );
    dataflow::apply_terminator(&mut state, &block.terminator.kind, paths);
}

fn check_statement_reads(
    body: &MirBody,
    stmt: &Statement,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    let span = stmt.span.clone();
    match &stmt.kind {
        StatementKind::Assign { rvalue, .. } => {
            check_rvalue(body, rvalue, span.as_ref(), state, paths, reported, diags);
        },
        StatementKind::Call { args, .. } => {
            for arg in args {
                check_value(body, arg, span.as_ref(), state, paths, reported, diags);
            }
        },
        StatementKind::Drop { .. } | StatementKind::DropIf { .. } => {
            // Compiler-inserted — never reads from the user's perspective.
        },
        // Internal drop-elaboration variants; no user-visible reads.
        StatementKind::Deinit { .. }
        | StatementKind::DeinitIf { .. }
        | StatementKind::SetDeinitFlag { .. }
        | StatementKind::ScopeLive(_) => {},
    }
}

fn check_terminator_reads(
    body: &MirBody,
    term: &TerminatorKind,
    site: Option<&Span>,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    match term {
        TerminatorKind::Return(v) | TerminatorKind::Branch { condition: v, .. } => {
            check_value(body, v, site, state, paths, reported, diags);
        },
        TerminatorKind::Switch { discriminant, .. } => {
            check_place_read(body, discriminant, site, state, paths, reported, diags);
        },
        TerminatorKind::Jump(_)
        | TerminatorKind::Panic(_)
        | TerminatorKind::Unreachable => {},
    }
}

fn check_rvalue(
    body: &MirBody,
    rv: &Rvalue,
    use_span: Option<&Span>,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    match rv {
        // Copy / Move are unambiguous reads — observing a moved value is a
        // bug. `Ref` is also a read (an immutable borrow of an uninit
        // place is semantically nonsensical, so anywhere `Ref(p)` shows
        // up in lowered code it's intended as a read). `RefMut` is
        // overloaded: in the read-modify shape it's a read, but in the
        // out-parameter init shape (`File.init(ref var %t, fd)`) it
        // initializes a still-uninit place. We don't yet have `&out T`
        // to distinguish those, so the heuristic is "RefMut is a read
        // iff `p` is at least maybe-init coming in" — see
        // `check_borrow_read`.
        Rvalue::Copy(p) | Rvalue::Move(p) | Rvalue::Ref(p) => {
            check_place_read(body, p, use_span, state, paths, reported, diags);
        },
        Rvalue::RefMut(p) => {
            check_borrow_read(body, p, use_span, state, paths, reported, diags);
        },
        Rvalue::Const(_) => {},
        Rvalue::Op1 { arg, .. } => {
            check_value(body, arg, use_span, state, paths, reported, diags);
        },
        Rvalue::Op2 { lhs, rhs, .. } => {
            check_value(body, lhs, use_span, state, paths, reported, diags);
            check_value(body, rhs, use_span, state, paths, reported, diags);
        },
        Rvalue::Op3 { a, b, c, .. } => {
            check_value(body, a, use_span, state, paths, reported, diags);
            check_value(body, b, use_span, state, paths, reported, diags);
            check_value(body, c, use_span, state, paths, reported, diags);
        },
        Rvalue::Construct { fields, .. } => {
            for (_, v) in fields {
                check_value(body, v, use_span, state, paths, reported, diags);
            }
        },
        Rvalue::Tuple(vs) | Rvalue::ArrayLiteral { values: vs, .. } => {
            for v in vs {
                check_value(body, v, use_span, state, paths, reported, diags);
            }
        },
        Rvalue::EnumVariant { payload, .. } => {
            for v in payload {
                check_value(body, v, use_span, state, paths, reported, diags);
            }
        },
        Rvalue::ApplyPartial { captures, .. } => {
            for v in captures {
                check_value(body, v, use_span, state, paths, reported, diags);
            }
        },
    }
}

fn check_value(
    body: &MirBody,
    v: &Value,
    use_span: Option<&Span>,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    match v {
        // See the comment on [`check_rvalue`] for the Ref vs RefMut split.
        Value::Copy(p) | Value::Move(p) | Value::Ref(p) => {
            check_place_read(body, p, use_span, state, paths, reported, diags);
        },
        Value::RefMut(p) => {
            check_borrow_read(body, p, use_span, state, paths, reported, diags);
        },
        Value::Const(_) => {},
    }
}

/// Mutable-borrow read check. Mirrors [`check_place_read`] but skips
/// when `p` is fully uninit — that shape is the out-parameter init
/// pattern (`File.init(ref var %t, fd)` writes into the uninit `%t`),
/// and reporting it would false-positive every stdlib initializer.
/// Once the MIR distinguishes `&out T` from `&var T` this special-case
/// goes away.
fn check_borrow_read(
    body: &MirBody,
    place: &Place,
    use_span: Option<&Span>,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    if let Some(path) = paths.lookup_place(place)
        && !state.is_maybe_init(path)
    {
        return;
    }
    check_place_read(body, place, use_span, state, paths, reported, diags);
}

fn check_place_read(
    body: &MirBody,
    place: &Place,
    use_span: Option<&Span>,
    state: &InitState,
    paths: &MovePathSet,
    reported: &mut HashSet<MovePathId>,
    diags: &mut Diagnostics,
) {
    let Some(path) = paths.lookup_place(place) else {
        return;
    };
    if reported.contains(&path) {
        return;
    }
    if state.is_definitely_init(path) {
        return;
    }
    // Suppress when the path was never actually moved on any reaching CFG
    // path. A `may_init=true, def_init=false, was_moved=false` shape comes
    // from CFG joins over branches that bind the local in only some arms
    // (e.g. `while let .Some(x) = … { … }` joins the Some arm — which
    // binds `x` — with the None arm — which doesn't — before the loop
    // body). The dataflow can't see that the body is only reached via the
    // Some arm, but there's no actual move to complain about. The
    // never-initialised case (`may_init=false, was_moved=false`) is
    // covered by other analyzers (E102 / E105) that operate on the HIR.
    if !state.was_moved(path) {
        return;
    }
    let may_init = state.is_maybe_init(path);
    let local = paths.get(path).local;
    let local_name = body.locals[local.index()].name.clone();
    let kind = if may_init {
        MoveDiagKind::MaybeMoved
    } else {
        MoveDiagKind::UseAfterMove
    };
    let use_site = use_span.cloned().unwrap_or_else(|| fallback_span(body));
    let move_site = state.move_site(path).cloned();
    diags.diags.push(MoveDiag {
        kind,
        local_name,
        use_site,
        move_site,
    });
    reported.insert(path);
}

/// Pick a stable span when the read site has no `Statement.span` plumbed
/// through yet. Walks the function body for any statement with a recorded
/// span and falls back to a synthetic file-start span as last resort.
fn fallback_span(body: &MirBody) -> Span {
    for block in &body.blocks {
        for stmt in &block.stmts {
            if let Some(s) = &stmt.span {
                return s.clone();
            }
        }
    }
    Span::synthetic(0)
}
