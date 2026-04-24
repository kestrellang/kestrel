# kestrel-hecs Architecture

Hierarchical Entity Component System with incremental queries, designed for use as a compiler's semantic database.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `World` | `world.rs` | Central database. Owns entities, components, hierarchy, queries, accumulators |
| `Entity` | `entity.rs` | Compact runtime handle (u32 index). Cheap to copy and compare |
| `Component` | `component.rs` | Trait alias for `Any + Clone + 'static`. Any cloneable type qualifies |
| `ComponentStore` | `component.rs` | Type-erased column storage. Each component type gets its own dense column |
| `QueryFn` | `query.rs` | Trait for memoized queries with automatic dependency tracking |
| `QueryContext` | `query.rs` | Read-only view of the world during query phase. Records dependencies |
| `ChangeSet` | `change.rs` | Tracks which entities changed per revision. Supports fingerprint backdating |
| `Fingerprint` | `fingerprint.rs` | 128-bit content hash for early cutoff (same content = skip re-execution) |
| `AccumulatorStore` | `accumulator.rs` | Side-effect storage for diagnostics, warnings, etc. |
| `Hierarchy` | `world.rs` | Parent-child entity relationships |
| `Revision` | `world.rs` | Monotonic counter identifying a compilation cycle |

## Lifecycle

A typical compilation cycle follows this pattern:

```
World::new()
    │
    ▼
begin_revision()          ── advances revision, clears change set
    │
    ▼
spawn()                   ── allocate a fresh entity
set(entity, component)    ── attach components, marks entity changed
set_parent(child, parent) ── build hierarchy
    │
    ▼
query_context()           ── borrow world immutably for query phase
    │
    ▼
ctx.query(MyQuery { .. }) ── execute queries with memoization
ctx.get::<T>(entity)      ── read components (records dependency)
ctx.accumulate(value)     ── push side-effect values
    │
    ▼
begin_revision()          ── next cycle (caches persist for reuse)
```

**Mutation and query phases alternate.** During mutation, the world is `&mut self`. During queries, it's `&self` via `QueryContext`. This separation ensures queries see a consistent snapshot of the data.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Crate root, re-exports primary types |
| `entity.rs` | `Entity` handle (compact u32 index) |
| `component.rs` | `Component` trait, `TypedColumn` sparse-dense storage, `ComponentStore` |
| `world.rs` | `World` database, `Hierarchy`, `Revision`, `snapshot()` |
| `query.rs` | `QueryFn` trait, `QueryContext`, memoization with dependency tracking |
| `change.rs` | `ChangeSet` with fingerprint backdating for early cutoff |
| `fingerprint.rs` | `Fingerprint` 128-bit hash via SipHash-2-4 |
| `accumulator.rs` | `AccumulatorStore` for side-effect values (diagnostics, etc.) |

## Storage Design

Components use a **sparse-dense** pattern per type:

```
TypedColumn<Health>:
  entity_to_index: { Entity(0) → 0, Entity(2) → 1 }
  dense:           [(Entity(0), Health(100)), (Entity(2), Health(50))]
```

- **O(1) lookup** via the entity-to-index map
- **Cache-friendly iteration** over the dense vec
- **Swap-remove** on deletion preserves dense packing

`ComponentStore` holds one `Box<dyn AnyColumn>` per component `TypeId`, giving type-erased access while preserving type safety through downcasting.

## Query Memoization

Queries implement `QueryFn` and are memoized by their `Hash + Eq` identity:

1. **Cache hit, same revision**: return cached value immediately
2. **Cache hit, stale**: check if dependencies changed. If not, mark verified and return cached
3. **Cache miss or deps changed**: execute query, record new dependencies, store result
4. **Backdating**: if re-execution produces the same fingerprint as before, keep the old `changed_at` revision so dependents don't needlessly re-execute

## Snapshot System

`World::snapshot()` creates a structural clone for reuse across compilations (e.g., stdlib persistence in tests). See [snapshots.md](snapshots.md) for details.
