# Kestrel Incremental Compilation Architecture

## Overview

Incremental, ECS-based compilation. The compiler persists a `World` across compilations.
Each compilation cycle updates source inputs, rebuilds only changed declarations, and
re-executes only the queries whose dependencies changed.

Approach: salsa-inspired custom ECS — borrow salsa's key ideas (dependency tracking,
early cutoff/backdating, fingerprinting) as plain Rust data structures with a modular
query system (no monolithic `dyn Db` trait).

---

## Pipeline

Two kinds of phases: **mutation** (write to World) and **query** (read from World).

```
MUTATION PHASES:
  1. Set source inputs     — world.set(file, SourceText(new_source))
  2. Lex + Parse           — queries (pure: source text in, syntax tree out)
  3. Builder               — creates/updates entities from syntax
  4. Change detection      — fingerprint declarations, backdate unchanged

QUERY PHASES:
  5. Binder queries        — ScopeFor, TypeFor, VisibleChildren, etc.
  6. Type inference         — constraint generation + solving per code block
  7. Analyzer queries       — conformance, visibility, etc.
  8. Collect diagnostics
```

---

## Entity Identification

Two-level scheme: stable keys for cross-compilation identity, compact handles for runtime.

```rust
/// Stable identity derived from source structure.
/// Survives across compilations — "Main.Point.Struct" is the same
/// entity whether it was compiled 1 second or 1 hour ago.
struct EntityKey {
    path: Vec<String>,       // e.g. ["Main", "Point", "distance"]
    kind: EntityKind,        // disambiguates struct Foo vs func Foo
}

/// Compact runtime handle. u32 index into the World's entity table.
/// NOT stable across compilations — use EntityKey for persistence.
struct Entity(u32);

enum EntityKind {
    SourceFile, Module, Struct, Enum, EnumCase, Protocol,
    Function, Initializer, Deinit, Field, Getter, Setter,
    Subscript, TypeAlias, AssociatedType, Extension, Import,
    TypeParameter,
}
```

Declaration-level entities get stable EntityKeys. Sub-declaration artifacts (expressions,
statements, locals) use ephemeral arena indices within their owning declaration — entire
declarations are recomputed atomically.

---

## Component Storage

Typed column storage (struct-of-arrays). Replaces `Vec<Arc<dyn Behavior>>` + downcasting.

```rust
trait Component: Any + Send + Sync + Clone + 'static {}

/// Dense column for one component type.
struct TypedColumn<T: Component> {
    entity_to_index: HashMap<Entity, usize>,  // sparse -> dense
    dense: Vec<(Entity, T)>,                  // cache-friendly iteration
}

/// All columns, keyed by TypeId.
struct ComponentStore {
    columns: HashMap<TypeId, Box<dyn AnyColumn>>,
}
```

### Concrete Components

```
BUILD phase (from syntax):
  SourceText(String)                                    — file source code
  SyntaxFragment(SyntaxNode)                            — declaration syntax node
  NameComponent { name, name_span, full_span }          — identity
  Visibility { level, scope_entity }                    — access control
  StaticMarker                                          — static member
  Generics { type_params: Vec<Entity> }                 — generic parameters
  ComputedPropertyMarker, ConcreteTypeMarker, etc.      — kind-group markers

BIND phase (from resolution):
  CallableSignature { params, return_type, receiver }   — function/method shape
  ResolvedType(Ty)                                      — resolved type annotation
  ResolvedBody { body: CodeBlock }                      — resolved function body
  Conformances { protocols: Vec<Ty> }                   — protocol conformances
  ExtensionTarget { target_entity: Entity }             — what an extension extends

INFER phase:
  InferenceSolution { substitutions: HashMap<TyId, Ty> }
```

---

## The World

Central compilation database. Persists across compilations.

```rust
struct Revision(u64);

struct World {
    revision: Revision,
    entities: Vec<EntityRecord>,                // dense entity table
    key_to_entity: HashMap<EntityKey, Entity>,  // stable key -> handle
    components: ComponentStore,
    queries: QueryStorage,                      // type-erased memoization
    hierarchy: Hierarchy,                       // parent/child relationships
    changes: ChangeSet,                         // fingerprint-based change tracking
    diagnostics: DiagnosticAccumulator,         // salsa-style accumulators
}

struct EntityRecord {
    key: EntityKey,
    last_changed: Revision,
    alive: bool,
}

struct Hierarchy {
    parent: HashMap<Entity, Entity>,
    children: HashMap<Entity, Vec<Entity>>,
}
```

---

## Change Detection

Fingerprint-based, at declaration granularity.

```rust
/// 128-bit hash for change detection.
struct Fingerprint { lo: u64, hi: u64 }

struct ChangeSet {
    changed: HashSet<Entity>,
    previous_fingerprints: HashMap<Entity, Fingerprint>,
    current_fingerprints: HashMap<Entity, Fingerprint>,
}
```

After the build phase, fingerprint each declaration's syntax text. If it matches the
previous revision, "backdate" the entity (remove from changed set). Downstream queries
see it as unchanged and return cached results — this is early cutoff.

---

## Query System

Modular: queries are structs implementing `QueryFn`. Each crate defines its own. No
shared database trait required.

```rust
/// Any crate can define queries by implementing this.
trait QueryFn: Hash + Eq + Clone + 'static {
    type Output: Clone + Hash + 'static;
    fn execute(&self, ctx: &QueryContext) -> Self::Output;
}

/// Tracks dependencies automatically during query execution.
struct QueryContext<'a> {
    world: &'a World,
    deps: RefCell<Vec<Dependency>>,            // accumulated deps
    active_queries: RefCell<Vec<ActiveQuery>>,  // cycle detection
}

enum Dependency {
    ComponentRead { entity: Entity, component_type: TypeId },
    Query { query_type: TypeId, key_hash: u64 },
}
```

### Memoization

```rust
struct MemoEntry<V> {
    value: V,
    fingerprint: Fingerprint,   // for early cutoff
    deps: Vec<Dependency>,      // what was read during execution
    computed_at: Revision,
    verified_at: Revision,      // when last confirmed still valid
    changed_at: Revision,       // when the VALUE last changed (backdating)
}

/// Type-erased. Each query type Q gets HashMap<Q, MemoEntry<Q::Output>>.
struct QueryStorage {
    stores: RefCell<HashMap<TypeId, Box<dyn Any>>>,
}
```

A memo is valid if `verified_at >= current_revision`, OR deps walk confirms nothing
changed. On re-execution with same fingerprint, backdate: `changed_at` stays old,
so this query's dependents also skip re-execution.

---

## Dependency Graph

```rust
struct DependencyGraph {
    /// Forward: "if X changes, what depends on X?"
    forward: HashMap<Dependency, HashSet<DependentQuery>>,
    /// Reverse: "what does Q depend on?" (cleanup on re-execution)
    reverse: HashMap<DependentQuery, Vec<Dependency>>,
}
```

Invalidation: BFS through forward edges from changed entities. Queries re-execute
lazily (on demand), not eagerly. Cycles detected at runtime via active-query stack.

---

## Diagnostics

Salsa accumulator pattern. Queries push diagnostics as side effects.

```rust
struct DiagnosticAccumulator {
    by_query: HashMap<DependentQuery, Vec<Diagnostic>>,
    by_entity: HashMap<Entity, Vec<Diagnostic>>,
}
```

When a query re-executes, its old diagnostics are cleared first. Each diagnostic is
owned by exactly one query — no deduplication needed.

---

## Orchestration

```rust
fn compile_incremental(world: &mut World, source_updates: Vec<(String, String)>) {
    // --- Mutation phases ---
    world.begin_revision();

    // 1. Update source inputs
    for (name, source) in &source_updates {
        let entity = world.intern_entity(EntityKey::source_file(name));
        world.set(entity, SourceText(source.clone()));
    }

    // 2. Builder: create declaration entities from syntax
    for file_entity in world.query_component::<SourceText>() {
        build_file(world, file_entity);
    }

    // 3. Fingerprint declarations, backdate unchanged
    for decl in world.query_component::<SyntaxFragment>() {
        let fp = Fingerprint::of(&decl.syntax_text());
        world.changes_mut().record_fingerprint(decl, fp);
    }

    // --- Query phases (immutable world) ---
    let ctx = QueryContext::new(&world);

    // 4-6. Queries cascade lazily. Unchanged declarations short-circuit.
    for &decl in &all_declarations {
        ctx.query(TypeFor { entity: decl });
        ctx.query(BindBody { entity: decl });
    }

    // 7. Collect diagnostics
    let diagnostics = world.diagnostics().all().collect();
}
```
