# World Snapshots

## Overview

`World::snapshot()` creates a structural clone of a world for reuse across compilations. The primary use case is **test stdlib persistence**: build the standard library once into a world, then snapshot it for each test so every test starts from a pre-built stdlib without recompiling it.

```
                    snapshot()              snapshot()
 [Build stdlib] ──────────────> [Test 1] ──────────────> [Test 2] ...
       |                           |                        |
   Original World            Cloned World              Cloned World
   (kept pristine)        (mutated by test)         (mutated by test)
```

## How It Works

Snapshot splits the world into two categories:

**Cloned** (structural data):
- `entities` — entity metadata (keys, liveness, change revision)
- `key_to_entity` — stable key-to-handle mapping
- `components` — all typed component columns (via `clone_box()`)
- `hierarchy` — parent/child relationships
- `changes` — change tracking state and fingerprints

**Fresh** (rebuilt lazily):
- `queries` — empty `QueryStorage`, repopulates on first query call
- `accumulators` — empty `AccumulatorStore`, repopulates during queries

Query caches are intentionally dropped because they contain type-erased memoized results tied to the original world's execution history. Since the snapshot's entity data is identical, queries re-execute once on first access and then memoize normally.

The snapshot keeps the same `revision` counter so `begin_revision()` advances naturally.

## Usage

```rust
// Build stdlib once
let mut stdlib_world = World::new();
stdlib_world.begin_revision();
// ... intern entities, set components for all stdlib symbols ...

// For each test: snapshot, then compile the test into the snapshot
let mut test_world = stdlib_world.snapshot();
test_world.begin_revision();
let e = test_world.intern_entity(EntityKey::root("test.ks", 0));
test_world.set(e, MyComponent { ... });

let ctx = test_world.query_context();
let result = ctx.query(MyQuery { entity: e });
// stdlib_world is unaffected
```

## clone_box Pattern

`ComponentStore` holds `HashMap<TypeId, Box<dyn AnyColumn>>`. Since `Box<dyn Trait>` doesn't implement `Clone`, each column type provides a `clone_box()` method on the `AnyColumn` trait:

```rust
trait AnyColumn: Any {
    // ...
    fn clone_box(&self) -> Box<dyn AnyColumn>;
}

impl<T: Component> AnyColumn for TypedColumn<T> {
    fn clone_box(&self) -> Box<dyn AnyColumn> {
        Box::new(TypedColumn {
            entity_to_index: self.entity_to_index.clone(),
            dense: self.dense.clone(),
        })
    }
}
```

This works because `T: Component` requires `Clone`. The manual `Clone` impl on `ComponentStore` iterates all columns and calls `clone_box()` on each.

## Design Notes

**"Clone for now" approach.** The current implementation does a full deep clone of all structural data. This is simple and correct. The `snapshot()` API is designed so that a future **layered/COW world** implementation could replace the clone with a cheap reference-counted layer without changing callers.

**Performance.** Clone cost is proportional to entity count and total component data. For stdlib-sized worlds (~thousands of entities), this is fast enough. If it becomes a bottleneck, the layered approach would share unchanged data via `Arc`.

**Revision continuity.** The snapshot preserves the revision counter so that `begin_revision()` on the snapshot produces the next sequential revision. This means change tracking and query invalidation work correctly across the original-to-snapshot boundary.
