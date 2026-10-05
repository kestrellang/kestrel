//! Durable invalidation: a memo is stale whenever an entity it read was
//! changed after the memo was last verified — no matter how many revisions
//! later it is read, and no matter whether the change came before or after
//! a query in the same revision.
//!
//! Regression tests for the hECS review finding "invalidation only lasts one
//! revision" (it subsumes audit F20): `deps_unchanged` used to consult the
//! per-revision `ChangeSet`, which `begin_revision` clears, so
//! (a) a change made in a revision where nothing read the entity was
//!     forgotten one revision later, and
//! (b) a change made after a query, before the next `begin_revision`, was
//!     never seen (the memo was "already verified this revision").
//! The LSP hits (b) by despawning a file's entities before it calls
//! `begin_revision`, and (a) whenever a request only re-parses one file.

use kestrel_hecs::{Entity, QueryContext, QueryFn, World};

#[derive(Clone)]
struct Val(i32);

#[derive(Clone, PartialEq, Eq, Hash)]
struct Read {
    e: Entity,
}

impl QueryFn for Read {
    type Output = i32;
    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        ctx.get::<Val>(self.e).map(|v| v.0).unwrap_or(-1)
    }
}

/// Depends on `Read` only through a sub-query.
#[derive(Clone, PartialEq, Eq, Hash)]
struct Doubled {
    e: Entity,
}

impl QueryFn for Doubled {
    type Output = i32;
    fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
        ctx.query(Read { e: self.e }) * 2
    }
}

fn world_with(value: i32) -> (World, Entity) {
    let mut world = World::new();
    world.begin_revision();
    let e = world.spawn();
    world.set(e, Val(value));
    (world, e)
}

#[test]
fn change_in_a_revision_nobody_reads_is_not_lost() {
    let (mut world, e) = world_with(1);
    assert_eq!(world.query_context().query(Read { e }), 1);

    world.begin_revision();
    world.set(e, Val(2)); // no query runs in this revision

    world.begin_revision();
    assert_eq!(world.query_context().query(Read { e }), 2);
}

#[test]
fn mutation_after_a_query_in_the_same_revision_is_seen() {
    let (mut world, e) = world_with(1);
    assert_eq!(world.query_context().query(Read { e }), 1);

    world.set(e, Val(5)); // no begin_revision in between
    assert_eq!(world.query_context().query(Read { e }), 5);
}

#[test]
fn despawn_before_begin_revision_invalidates() {
    // The LSP's order: unbuild (despawn) first, then begin_revision.
    let (mut world, e) = world_with(1);
    assert_eq!(world.query_context().query(Read { e }), 1);

    world.despawn(e);
    world.begin_revision();
    assert_eq!(world.query_context().query(Read { e }), -1);
}

#[test]
fn stale_input_reaches_through_a_cached_sub_query() {
    let (mut world, e) = world_with(3);
    assert_eq!(world.query_context().query(Doubled { e }), 6);

    world.begin_revision();
    world.set(e, Val(4));
    world.begin_revision();
    world.begin_revision(); // several revisions later
    assert_eq!(world.query_context().query(Doubled { e }), 8);
}

#[test]
fn unchanged_inputs_still_hit_the_cache() {
    let (mut world, e) = world_with(7);
    let other = world.spawn();
    world.set(other, Val(0));
    assert_eq!(world.query_context().query(Doubled { e }), 14);
    let executed = world.query_exec_count();

    // Unrelated mutations — after a query, in an unobserved revision, and
    // across several revisions — must not re-execute `Doubled`/`Read` for `e`.
    world.set(other, Val(1));
    world.begin_revision();
    world.set(other, Val(2));
    world.begin_revision();
    assert_eq!(world.query_context().query(Doubled { e }), 14);
    assert_eq!(world.query_exec_count(), executed);
}
