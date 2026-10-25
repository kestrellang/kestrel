//! F22, part 2: state that must be rolled back when a query unwinds.
//!
//! `f22_unwind_repro.rs` covers the active-query stack. These cover the two
//! pieces of bookkeeping `ensure_fresh` / `execute_query` mutate *before*
//! running user code and can no longer put back by hand:
//!   * the tentative `verified_at` mark that `deps_unchanged` is supposed to
//!     make good on, and
//!   * the query's previously accumulated diagnostics.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use kestrel_hecs::{Entity, QueryContext, QueryFn, World};

#[derive(Clone)]
struct Value(i32);

thread_local! {
    static SHOULD_PANIC: Cell<bool> = const { Cell::new(false) };
}

fn arm_panic(on: bool) {
    SHOULD_PANIC.with(|c| c.set(on));
}

fn panic_if_armed(what: &str) {
    if SHOULD_PANIC.with(|c| c.get()) {
        panic!("ICE: simulated compiler panic inside {what}");
    }
}

// ===== 1. the tentative verified_at mark =====

/// Leaf query. Panics on demand — standing in for an ICE reached while a
/// *cached* dependent is being re-verified.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Leaf {
    entity: Entity,
}

impl QueryFn for Leaf {
    type Output = i32;

    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        panic_if_armed("Leaf");
        ctx.get::<Value>(self.entity).map(|v| v.0).unwrap_or(0)
    }
}

/// Memoized dependent. Its cached deps include `Leaf`, so verifying it in a
/// new revision runs `Leaf` again — which is where the panic lands.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Doubled {
    entity: Entity,
}

impl QueryFn for Doubled {
    type Output = i32;

    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        ctx.query(Leaf {
            entity: self.entity,
        }) * 2
    }
}

#[test]
fn panicking_verification_does_not_leave_a_query_marked_verified() {
    let mut world = World::new();
    world.begin_revision();
    let e = world.spawn();
    world.set(e, Value(1));

    // Revision 1: populate the memo for Doubled (and Leaf).
    {
        let ctx = world.query_context();
        assert_eq!(ctx.query(Doubled { entity: e }), 2);
    }

    // Revision 2: the input changed, so verifying Doubled must re-run Leaf.
    world.begin_revision();
    world.set(e, Value(5));

    let ctx = world.query_context();

    arm_panic(true);
    let first = catch_unwind(AssertUnwindSafe(|| ctx.query(Doubled { entity: e })));
    assert!(first.is_err(), "Leaf should have panicked during verification");
    arm_panic(false);

    // Same revision, same context — exactly what `infer_all` and the LSP
    // worker do after catching. `ensure_fresh` marked Doubled (and Leaf)
    // `verified_at = rev2` *before* calling `deps_unchanged`; that promise
    // was never kept, so the mark must have been rolled back. Otherwise this
    // returns the stale 2 without re-executing anything.
    let second = ctx.query(Doubled { entity: e });
    assert_eq!(
        second, 10,
        "a query whose verification panicked was trusted as verified"
    );
}

// ===== 2. the query's accumulated values =====

#[derive(Clone, PartialEq, Eq, Hash)]
struct Emit {
    entity: Entity,
    /// Push a value before panicking, so the roll-back has something to
    /// discard as well as something to restore.
    partial: bool,
}

impl QueryFn for Emit {
    type Output = i32;

    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        // Only the run that is about to abort writes a partial value.
        if self.partial && SHOULD_PANIC.with(|c| c.get()) {
            ctx.accumulate("half-finished diagnostic".to_string());
        }
        panic_if_armed("Emit");
        ctx.accumulate("diagnostic from the good run".to_string());
        ctx.get::<Value>(self.entity).map(|v| v.0).unwrap_or(0)
    }
}

fn accumulated_after_panicking_rerun(partial: bool) -> Vec<String> {
    let mut world = World::new();
    world.begin_revision();
    let e = world.spawn();
    world.set(e, Value(1));

    // Revision 1: a clean run files one diagnostic under Emit's key.
    {
        let ctx = world.query_context();
        arm_panic(false);
        ctx.query(Emit { entity: e, partial });
    }
    assert_eq!(world.accumulated::<String>().len(), 1);

    // Revision 2: the input changed, so Emit re-executes — and ICEs. Its
    // revision-1 diagnostic was already cleared to make room for the run
    // that never finished.
    world.begin_revision();
    world.set(e, Value(2));
    {
        let ctx = world.query_context();
        arm_panic(true);
        let r = catch_unwind(AssertUnwindSafe(|| ctx.query(Emit { entity: e, partial })));
        assert!(r.is_err());
        arm_panic(false);
    }

    world.accumulated::<String>()
}

#[test]
fn panicking_reexecution_keeps_the_previous_revisions_diagnostics() {
    let diags = accumulated_after_panicking_rerun(false);
    assert_eq!(
        diags,
        vec!["diagnostic from the good run".to_string()],
        "a caught ICE deleted a previously-valid diagnostic"
    );
}

#[test]
fn panicking_reexecution_discards_its_own_half_written_diagnostics() {
    let diags = accumulated_after_panicking_rerun(true);
    assert_eq!(
        diags,
        vec!["diagnostic from the good run".to_string()],
        "the aborted run's partial output leaked into the accumulator"
    );
}
