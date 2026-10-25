//! Accumulated values follow the memos that filed them.
//!
//! - Audit F21: `World::snapshot` cloned the memos but started an empty
//!   accumulator store, so a query that verified as a cache hit in the
//!   snapshot never re-filed its diagnostics and they were simply gone.
//! - Audit F23: values were never pruned, so a query nothing demands any
//!   more — a deleted function's body check — kept reporting its errors.

use kestrel_hecs::{Entity, QueryContext, QueryFn, World};

#[derive(Clone)]
struct Val(i32);

/// Reads `e` directly and files one message about it.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Check {
    e: Entity,
}

impl QueryFn for Check {
    type Output = i32;
    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        let v = ctx.get::<Val>(self.e).map(|v| v.0).unwrap_or(-1);
        ctx.accumulate(format!("check {v}"));
        v
    }
}

/// Sees `e` only through `Check`, and files a message of its own.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Summary {
    e: Entity,
}

impl QueryFn for Summary {
    type Output = bool;
    fn execute(&self, ctx: &QueryContext<'_>) -> bool {
        let positive = ctx.query(Check { e: self.e }) > 0;
        ctx.accumulate(format!("summary {positive}"));
        positive
    }
}

fn messages(world: &World) -> Vec<String> {
    let mut all = world.accumulated::<String>();
    all.sort();
    all
}

fn world_with(values: &[i32]) -> (World, Vec<Entity>) {
    let mut world = World::new();
    world.begin_revision();
    let entities = values
        .iter()
        .map(|&v| {
            let e = world.spawn();
            world.set(e, Val(v));
            e
        })
        .collect();
    (world, entities)
}

#[test]
fn a_snapshot_keeps_the_values_of_its_cached_queries() {
    let (mut world, es) = world_with(&[7]);
    world.query_context().query(Summary { e: es[0] });
    world.begin_revision();

    let snap = world.snapshot();
    snap.query_context().query(Summary { e: es[0] });
    assert_eq!(snap.query_exec_count(), 0, "the snapshot should hit the cache");
    assert_eq!(messages(&snap), ["check 7", "summary true"]);
}

#[test]
fn despawning_an_entity_drops_its_readers_values() {
    let (mut world, es) = world_with(&[7, 8]);
    let ctx = world.query_context();
    ctx.query(Check { e: es[0] });
    ctx.query(Check { e: es[1] });

    world.despawn(es[0]);
    assert_eq!(messages(&world), ["check 8"]);
}

#[test]
fn a_reader_demanded_again_refiles_its_values() {
    let (mut world, es) = world_with(&[7]);
    world.query_context().query(Check { e: es[0] });

    world.despawn(es[0]);
    world.query_context().query(Check { e: es[0] });
    assert_eq!(messages(&world), ["check -1"]);
}

#[test]
fn an_indirect_reader_keeps_its_values() {
    // `Summary` reads the dead entity only through `Check`. It is not
    // guaranteed to re-run (`Check` could backdate), so its values must
    // survive the sweep; `Check`'s own are dropped.
    let (mut world, es) = world_with(&[7]);
    world.query_context().query(Summary { e: es[0] });

    world.despawn(es[0]);
    assert_eq!(messages(&world), ["summary true"]);
}
