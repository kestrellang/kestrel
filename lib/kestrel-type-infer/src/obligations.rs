//! The end of a body (bidi design P5).
//!
//! The walker (`generate.rs`) resolves members, calls, operators, patterns
//! and `.Case`s where it meets them and checks every introduction form
//! against its expectation, so what reaches the end of a body is only:
//!
//! - **pending obligations** — conformances (decided by the `kestrel-entail`
//!   kernel; `Unknown` while a var is open), applicability of members
//!   committed on an `Unknown` (`Constraint::Applicable`), equalities and
//!   coercions still waiting on an open var;
//! - **things that are open by the language's rules** — numeric literals are
//!   body-scoped (ruling D2) and take their default here, together with the
//!   operator rules that read them (Rules L and C for operands that became
//!   known later in the body).
//!
//! [`finish_body`] flushes the queue, applies the end-of-body defaults in a
//! fixed order (restarting after any progress), then reports what is left.

use kestrel_hir::body::HirBody;

use crate::ctx::InferCtx;
use crate::solver::{self, fixpoint};

/// One end-of-body default rule: returns true when it made progress.
type DefaultRule = fn(&mut InferCtx<'_>) -> bool;

/// The end-of-body defaults, in priority order. After any rule makes
/// progress the queue is flushed and the list restarts from the top, so a
/// later (weaker) rule only runs once every earlier one is exhausted.
const DEFAULT_RULES: &[(&str, DefaultRule)] = &[
    // An operator over a literal gets its result's *shape* from the
    // literal's default before defaulting (`100..=200` → `ClosedRange[ℓ]`),
    // so a generic bound on the result can still pin the literal.
    ("operator shape", solver::apply_operator_shape_projections),
    // Rule L for operands that became known after the walker passed the
    // operator.
    ("rule L", solver::apply_operator_literal_adoption),
    // Rule C, then literal defaults, holding back arguments of calls still
    // waiting on their receiver and interpolation accumulators.
    ("literal defaults (strict)", |ctx| solver::apply_literal_defaults(ctx, 0)),
    // A ref-initialized binding nothing else pinned takes its copy-out
    // decay; it can gate the literals the next rule would default.
    ("ref decay", solver::apply_ref_decay_defaults),
    // Interpolation accumulators default too (so `"\(n)".bytes` resolves).
    ("literal defaults (relaxed)", |ctx| solver::apply_literal_defaults(ctx, 1)),
];

/// Finish inference for a body: flush, default, report.
pub fn finish_body(ctx: &mut InferCtx<'_>, hir: &HirBody) {
    // P5 audit: which constraint kinds the walker left for the flush.
    for c in ctx.constraints.iter().filter(|_| kestrel_debug::is_enabled("p5")) {
        let kind = format!("{c:?}");
        let kind = kind.split([' ', '{', '(']).next().unwrap_or("");
        kestrel_debug::ktrace!("p5", "{kind}");
    }
    fixpoint(ctx);
    for c in ctx.constraints.iter().filter(|_| kestrel_debug::is_enabled("p5")) {
        let kind = format!("{c:?}");
        let kind = kind.split([' ', '{', '(']).next().unwrap_or("");
        kestrel_debug::ktrace!("p5", "after-flush {kind}");
    }
    apply_defaults(ctx);

    // Every call/member that could type a check-mode leaf has fired, so a
    // leaf typed only by its target (`none()`) may now unify with it.
    ctx.synth_gate = false;
    fixpoint(ctx);

    // Type-parameter defaults (`H = DefaultHasher`) for vars nothing pinned.
    if solver::apply_type_param_defaults(ctx) {
        fixpoint(ctx);
    }

    // Reports. Unresolved generic type params first, so their vars are
    // poisoned before cascades; the never-fallback (Rust's
    // `never_type_fallback`) before unsolved constraints; unsolved
    // constraints before the generic "could not infer type", so a specific
    // error wins.
    solver::report_unresolved_type_params(ctx);
    solver::default_never_fallback(ctx);
    solver::report_unsolved(ctx);
    // Refs are second-class: legal only as an expression's top-level type.
    solver::validate_ref_placement(ctx, hir);
    solver::report_unresolved_slots(ctx, hir);
    drop_cascades(ctx);
}

/// bidi §10: an error about an already-failed type is a cascade of an
/// earlier error — drop it, but only when the body reported a root cause.
/// A body whose every error is a cascade keeps them all: some type was
/// marked failed without its own report, and dropping them would let a
/// wrong program compile.
fn drop_cascades(ctx: &mut InferCtx<'_>) {
    if !ctx.error_is_cascade.iter().any(|&c| !c) {
        return;
    }
    let keep: Vec<bool> = ctx.error_is_cascade.iter().map(|&c| !c).collect();
    let mut i = 0;
    ctx.errors.retain(|_| {
        i += 1;
        keep[i - 1]
    });
    let mut i = 0;
    ctx.error_details.retain(|_| {
        i += 1;
        keep[i - 1]
    });
    ctx.error_is_cascade.retain(|&c| !c);
}

/// Apply [`DEFAULT_RULES`] until none makes progress.
fn apply_defaults(ctx: &mut InferCtx<'_>) {
    'restart: loop {
        for (name, rule) in DEFAULT_RULES {
            if rule(ctx) {
                kestrel_debug::ktrace!("defaults", "end-of-body rule '{name}' made progress");
                fixpoint(ctx);
                continue 'restart;
            }
        }
        return;
    }
}
