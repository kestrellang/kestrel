# kestrel-hecs Architecture

Hierarchical Entity Component System with incremental queries, designed for use as a compiler's semantic database.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `World` | `world.rs` | Central database. Owns entities, components, hierarchy, queries, accumulators |
| `Entity` | `entity.rs` | Compact runtime handle (u32 index). Cheap to copy and compare |
| `Component` | `component.rs` | Trait alias for `Any + Clone + Send + Sync + 'static`. Any cloneable, thread-safe type qualifies (query outputs and accumulated values carry the same `Send + Sync` bound, so `World` is `Send`; audit F42) |
| `ComponentStore` | `component.rs` | Type-erased column storage. Each component type gets its own dense column |
| `QueryFn` | `query.rs` | Trait for memoized queries with automatic dependency tracking |
| `QueryContext` | `query.rs` | Read-only view of the world during query phase. Records dependencies |
| `ChangeSet` | `change.rs` | Which entities changed in the current revision (informational; cleared by `begin_revision`). Invalidation reads `EntityRecord::last_changed` instead |
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

**Calling order is not load-bearing.** Every mutation stamps the entity's durable `EntityRecord::last_changed`, and invalidation compares that stamp against the memo's `verified_at` — so a change is seen no matter how many revisions pass before the next query. The first mutation after a `query_context()` was handed out starts a new revision automatically (`World::touch`), so mutating after a query without calling `begin_revision()` is also safe. `begin_revision()` remains the way to group a batch of edits into one revision.

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
2. **Cache hit, stale**: check if dependencies changed since `verified_at` — a component dependency by its entity's `last_changed`, a sub-query by its memo's `changed_at`. If none did, mark verified and return cached
3. **Cache miss or deps changed**: execute query, record new dependencies, store result
4. **Backdating**: if re-execution produces the same fingerprint as before, keep the old `changed_at` revision so dependents don't needlessly re-execute

## Cycle Recovery

A query that asks for its own key while it is still executing (directly or
through other queries) forms a **cycle**; the first of its queries on the stack
is the cycle **head**. By default a cycle panics ("Query cycle detected").
Recursive-type questions (is `List[T]` Copyable when `List` contains `List`?)
are cycles by nature, so a query can opt in through `QueryFn::cycle_mode`, in
the shape of Salsa's cycle handling:

| `CycleMode` | re-entry answers | the head |
|---|---|---|
| `Panic` (default) | — (panics) | — |
| `Fixpoint { initial }` | the head's provisional value, first `initial` | re-executes with its last output as the provisional value until the output's fingerprint stops changing |
| `Fallback { value }` | `value` | answers `value`; `on_cycle_fallback` runs in its scope (for a diagnostic) |

Rules the framework enforces:

- **Provisional results are never memoized directly.** Every frame above the
  head when it was re-entered (or above it when a provisional result was read)
  depends on the head. Its result is parked in `QueryContext::provisional`,
  answered to later readers in the same iteration, dropped when the head
  iterates, and committed when the head converges. A head that itself depends
  on an outer head is parked under the outer one, with its members.
- **One dependency list per cycle.** At commit every member (head included)
  gets the union of all members' deps minus the edges between members. Verifying
  any member therefore never re-enters the cycle, and a change to any input
  re-runs every member from `initial` — no stale "assumed" answers.
- **Termination is the query's job.** Outputs must walk a finite lattice
  monotonically from `initial` (copy semantics: Copyable → Cloneable →
  NotCopyable). The framework caps iterations (`MAX_CYCLE_ITERATIONS`) and
  ICEs past it.

Prefer this over a thread-local "currently computing" stack: such a stack is
invisible to dependency tracking, so a memo computed under its assumption is
never invalidated when the rest of the cycle changes.

## Snapshot System

`World::snapshot()` creates a structural clone for reuse across compilations (e.g., stdlib persistence in tests). See [snapshots.md](snapshots.md) for details.
