//! F22 diagnosis repro: the active-query stack is not unwind-safe.
//!
//! Models exactly what `CompilerDriver::infer_all` does: one shared
//! `QueryContext`, `catch_unwind` per unit of work, keep going.

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};

use kestrel_hecs::{Entity, QueryContext, QueryFn, World};

#[derive(Clone)]
struct Name(String);

thread_local! {
    static SHOULD_PANIC: Cell<bool> = const { Cell::new(true) };
}

/// A query that panics on its first execution and succeeds afterwards.
/// Stands in for any compiler ICE reached from inside a query.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Flaky {
    entity: Entity,
}

impl QueryFn for Flaky {
    type Output = Option<String>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        if SHOULD_PANIC.with(|c| c.replace(false)) {
            panic!("ICE: simulated compiler panic inside a query");
        }
        ctx.get::<Name>(self.entity).map(|n| n.0.clone())
    }
}

/// An outer query that calls `Flaky`, so the leaked stack entry is *not*
/// the query the host re-runs — it is a sub-query several frames down.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Outer {
    entity: Entity,
}

impl QueryFn for Outer {
    type Output = Option<String>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        ctx.query(Flaky {
            entity: self.entity,
        })
    }
}

/// After a caught panic the leaked `active` entry fabricates a
/// "Query cycle detected" panic for an entirely non-recursive query.
#[test]
fn caught_panic_leaves_active_stack_dirty_and_fabricates_a_cycle() {
    let mut world = World::new();
    world.begin_revision();
    let e = world.spawn();
    world.set(e, Name("Alice".into()));

    // ONE context shared across units of work — exactly infer_all's shape.
    let ctx = world.query_context();

    // Unit of work 1: panics; host catches and keeps going.
    let first = catch_unwind(AssertUnwindSafe(|| ctx.query(Outer { entity: e })));
    assert!(first.is_err(), "first unit of work should have panicked");

    // Unit of work 2: a fresh, non-recursive query on the same ctx.
    // Flaky no longer panics, so this must return Some("Alice").
    let second = catch_unwind(AssertUnwindSafe(|| ctx.query(Flaky { entity: e })));

    // Asserts the DESIRED behaviour. Today this fails with a fabricated
    // "Query cycle detected", because `Outer`/`Flaky` are still on
    // `QueryContext::active` after the unwind (no RAII guard at query.rs:378/:394).
    let msg = match second {
        Ok(v) => {
            assert_eq!(v, Some("Alice".to_string()));
            return;
        },
        Err(payload) => payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_default(),
    };
    panic!("F22: leaked active-query stack fabricated a cycle: {msg}");
}

/// A top-level `ctx.accumulate` after a caught panic is filed under the
/// leaked query key instead of the (0,0) sentinel, so re-running that
/// query's `clear_for_query` deletes an unrelated diagnostic.
#[test]
fn caught_panic_misattributes_later_top_level_accumulations() {
    SHOULD_PANIC.with(|c| c.set(true));

    let mut world = World::new();
    world.begin_revision();
    let e = world.spawn();
    world.set(e, Name("Bob".into()));

    {
        let ctx = world.query_context();
        let r = catch_unwind(AssertUnwindSafe(|| ctx.query(Outer { entity: e })));
        assert!(r.is_err());

        // Host-level diagnostic emitted OUTSIDE any query (cf.
        // kestrel-compiler/src/lib.rs:238, the MIR-verify path).
        ctx.accumulate("mir verify error".to_string());
    }
    assert_eq!(
        world.accumulated::<String>().len(),
        1,
        "diagnostic should be present right after emission"
    );

    // A later revision re-runs Outer/Flaky. `clear_for_query` for the
    // leaked key wipes the host diagnostic that was misfiled under it.
    world.begin_revision();
    world.set(e, Name("Carol".into()));
    {
        let ctx = world.query_context();
        let _ = ctx.query(Outer { entity: e });
    }

    let survived = world.accumulated::<String>().len();
    eprintln!("F22 accumulate misattribution: surviving diagnostics = {survived}");
    assert_eq!(
        survived, 1,
        "the host diagnostic was collected by a query's clear_for_query"
    );
}
