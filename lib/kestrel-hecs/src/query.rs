use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::accumulator::AccumulatorStore;
use crate::change::ChangeSet;
use crate::component::{Component, ComponentStore};
use crate::entity::Entity;
use crate::fingerprint::Fingerprint;
use crate::world::Revision;

/// Identifies a specific query invocation (type + key hash).
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct QueryKey {
    pub type_id: u64,
    pub key_hash: u64,
}

/// What a query depends on. Recorded automatically during execution.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Dependency {
    /// Read a component of type `component_type` from `entity`.
    Component {
        entity: Entity,
        component_type: TypeId,
    },
    /// Called a sub-query identified by its QueryKey.
    Query(QueryKey),
}

/// Trait for query functions. Any crate can define queries by implementing this.
///
/// Queries are structs containing their inputs. The struct's Hash/Eq are
/// used for memoization lookup. The `execute` method computes the result
/// using the `QueryContext` to read components and call sub-queries.
///
/// # Example
/// ```ignore
/// #[derive(Clone, PartialEq, Eq, Hash)]
/// struct TypeFor { entity: Entity }
///
/// impl QueryFn for TypeFor {
///     type Output = Option<ResolvedType>;
///     fn execute(&self, ctx: &QueryContext) -> Self::Output {
///         let syntax = ctx.get::<SyntaxFragment>(self.entity)?;
///         // ... resolve ...
///         Some(resolved)
///     }
/// }
/// ```
pub trait QueryFn: Hash + Eq + Clone + 'static {
    type Output: Clone + Hash + 'static;

    /// Execute the query. All component reads and sub-query calls go
    /// through `ctx`, which records dependencies automatically.
    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output;

    /// Debug description for cycle detection diagnostics.
    fn describe(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }
}

/// A memoized query result with dependency and revision tracking.
#[derive(Clone)]
struct MemoEntry<V> {
    value: V,
    /// Fingerprint of the value for early cutoff.
    fingerprint: Fingerprint,
    /// What this query read during execution.
    deps: Vec<Dependency>,
    /// When this result was computed.
    #[allow(dead_code)]
    computed_at: Revision,
    /// When this result was last verified as still valid.
    verified_at: Revision,
    /// When the VALUE last actually changed. If re-execution produces
    /// the same fingerprint, this stays at the old value (backdating),
    /// so dependents of this query also skip re-execution.
    changed_at: Revision,
}

/// Type-erased function that verifies a sub-query and returns its changed_at.
///
/// Used by `deps_unchanged` to recursively verify sub-query dependencies
/// without knowing the sub-query's concrete type.
type VerifierFn = Arc<dyn Fn(&QueryContext<'_>) -> Revision>;

/// Type-erased store that can be cloned. Pairs a `Box<dyn Any>` with a
/// clone function so we can duplicate query caches across snapshots.
struct ErasedStore {
    data: Box<dyn Any>,
    clone_fn: fn(&dyn Any) -> Box<dyn Any>,
}

impl ErasedStore {
    fn new<T: Clone + 'static>(value: T) -> Self {
        Self {
            data: Box::new(value),
            clone_fn: |any| {
                Box::new(
                    any.downcast_ref::<T>()
                        .expect("type mismatch in ErasedStore clone")
                        .clone(),
                )
            },
        }
    }

    fn clone_store(&self) -> Self {
        Self {
            data: (self.clone_fn)(&*self.data),
            clone_fn: self.clone_fn,
        }
    }
}

/// Type-erased memoization storage. Each query type Q gets its own
/// `FxHashMap<Q, MemoEntry<Q::Output>>`.
pub(crate) struct QueryStorage {
    stores: FxHashMap<TypeId, ErasedStore>,
    /// Type-erased verifiers keyed by QueryKey. Registered when a query
    /// is first computed. Called during dep verification to recursively
    /// check if sub-queries have changed.
    verifiers: FxHashMap<QueryKey, VerifierFn>,
}

impl QueryStorage {
    pub fn new() -> Self {
        Self {
            stores: FxHashMap::default(),
            verifiers: FxHashMap::default(),
        }
    }

    fn get_memo<Q: QueryFn>(&self, key: &Q) -> Option<&MemoEntry<Q::Output>> {
        self.stores
            .get(&TypeId::of::<Q>())
            .and_then(|s| s.data.downcast_ref::<FxHashMap<Q, MemoEntry<Q::Output>>>())
            .and_then(|map| map.get(key))
    }

    fn insert_memo<Q: QueryFn>(&mut self, key: Q, entry: MemoEntry<Q::Output>) {
        self.stores
            .entry(TypeId::of::<Q>())
            .or_insert_with(|| ErasedStore::new(FxHashMap::<Q, MemoEntry<Q::Output>>::default()))
            .data
            .downcast_mut::<FxHashMap<Q, MemoEntry<Q::Output>>>()
            .expect("type mismatch in query storage")
            .insert(key, entry);
    }

    fn update_verified<Q: QueryFn>(&mut self, key: &Q, revision: Revision) {
        if let Some(store) = self.stores.get_mut(&TypeId::of::<Q>())
            && let Some(map) = store
                .data
                .downcast_mut::<FxHashMap<Q, MemoEntry<Q::Output>>>()
            && let Some(memo) = map.get_mut(key)
        {
            memo.verified_at = revision;
        }
    }

    fn register_verifier(&mut self, key: QueryKey, verifier: VerifierFn) {
        self.verifiers.insert(key, verifier);
    }
}

impl Clone for QueryStorage {
    fn clone(&self) -> Self {
        let stores = self
            .stores
            .iter()
            .map(|(&k, v)| (k, v.clone_store()))
            .collect();
        Self {
            stores,
            verifiers: self.verifiers.clone(),
        }
    }
}

impl Default for QueryStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks an active query on the call stack for cycle detection.
#[derive(Clone, Debug)]
struct ActiveQuery {
    key: QueryKey,
    /// Debug label for cycle diagnostics (from QueryFn::describe).
    label: String,
}

/// Context for query execution. Provides read access to the world
/// while automatically tracking dependencies.
///
/// Created by `World::query_context()` for the query (read) phase.
/// All component reads and sub-query calls record dependencies, enabling
/// incremental recomputation on subsequent revisions.
pub struct QueryContext<'a> {
    pub(crate) revision: Revision,
    pub(crate) components: &'a ComponentStore,
    pub(crate) changes: &'a ChangeSet,
    pub(crate) hierarchy: &'a crate::world::Hierarchy,
    pub(crate) queries: &'a RefCell<QueryStorage>,
    pub(crate) accumulators: &'a RefCell<AccumulatorStore>,
    /// Counter for actual query executions (not cache hits).
    pub(crate) exec_count: &'a Cell<u64>,
    /// Dependencies recorded during the current query's execution.
    deps: RefCell<Vec<Dependency>>,
    /// Call stack for cycle detection.
    active: RefCell<Vec<ActiveQuery>>,
}

impl<'a> QueryContext<'a> {
    pub(crate) fn new(
        revision: Revision,
        components: &'a ComponentStore,
        changes: &'a ChangeSet,
        hierarchy: &'a crate::world::Hierarchy,
        queries: &'a RefCell<QueryStorage>,
        accumulators: &'a RefCell<AccumulatorStore>,
        exec_count: &'a Cell<u64>,
    ) -> Self {
        Self {
            revision,
            components,
            changes,
            hierarchy,
            queries,
            accumulators,
            exec_count,
            deps: RefCell::new(Vec::new()),
            active: RefCell::new(Vec::new()),
        }
    }

    /// Read a component from an entity. Records a dependency.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&'a T> {
        self.deps.borrow_mut().push(Dependency::Component {
            entity,
            component_type: TypeId::of::<T>(),
        });
        self.components.get::<T>(entity)
    }

    /// Check if an entity has a component. Records a dependency.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.deps.borrow_mut().push(Dependency::Component {
            entity,
            component_type: TypeId::of::<T>(),
        });
        self.components.has::<T>(entity)
    }

    /// Get children of an entity. Records a dependency on the entity
    /// so the query re-fires when children are added or removed
    /// (`World::set_parent` and `Hierarchy::detach` both mark the
    /// parent as changed). Without this, despawning a child would
    /// leave queries that walked the parent's children list returning
    /// stale dead-entity IDs from cache.
    pub fn children_of(&self, entity: Entity) -> &'a [Entity] {
        self.deps.borrow_mut().push(Dependency::Component {
            entity,
            component_type: hierarchy_dep_type(),
        });
        self.hierarchy.children_of(entity)
    }

    /// Get parent of an entity. Records a dependency on the entity
    /// for the same reason as `children_of`.
    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        self.deps.borrow_mut().push(Dependency::Component {
            entity,
            component_type: hierarchy_dep_type(),
        });
        self.hierarchy.parent_of(entity)
    }

    /// Push a value into an accumulator (e.g. diagnostics).
    ///
    /// Prefer `ctx.throw(error)` (via the `ThrowDiagnostic` extension trait
    /// in kestrel-compiler) for reporting diagnostics — it enforces that
    /// errors go through `ToDiagnostic` for consistent formatting.
    /// Use `accumulate` directly only for non-diagnostic side-effects.
    pub fn accumulate<T: Clone + 'static>(&self, value: T) {
        // Associate with the current active query
        let active = self.active.borrow();
        let query_key = active.last().map(|a| a.key.clone()).unwrap_or(QueryKey {
            type_id: 0,
            key_hash: 0,
        });
        self.accumulators.borrow_mut().push(query_key, value);
    }

    /// Execute a sub-query with memoization and dependency tracking.
    pub fn query<Q: QueryFn>(&self, q: Q) -> Q::Output {
        let qk = query_key::<Q>(&q);

        // Record dependency on this sub-query
        self.deps.borrow_mut().push(Dependency::Query(qk.clone()));

        self.ensure_fresh(&q, &qk).0
    }

    /// Core query logic: verify from cache or re-execute.
    /// Returns (value, changed_at). Does NOT record a dependency —
    /// the caller (query() or a verifier) handles that.
    fn ensure_fresh<Q: QueryFn>(&self, q: &Q, qk: &QueryKey) -> (Q::Output, Revision) {
        // Phase 1: Check memo. The fast path (already verified this revision)
        // is by far the hottest, so return from it before cloning the deps
        // vector — for widely-used queries deps can be long, and cloning it
        // on every cache hit dominated batch-build profiles.
        let memo_info = {
            let storage = self.queries.borrow();
            match storage.get_memo::<Q>(q) {
                Some(memo) if memo.verified_at >= self.revision => {
                    return (memo.value.clone(), memo.changed_at);
                },
                Some(memo) => Some((memo.verified_at, memo.changed_at, memo.deps.clone())),
                None => None,
            }
        };

        if let Some((verified_at, changed_at, deps)) = memo_info {
            // Tentatively mark as verified to break verification cycles.
            // If deps check fails, we'll re-execute and overwrite the memo.
            self.queries
                .borrow_mut()
                .update_verified::<Q>(q, self.revision);

            // Check if all deps are still valid (may recursively verify sub-queries)
            if self.deps_unchanged(&deps, verified_at) {
                // Memo still valid — clone the value only now; cloning it
                // before verification would be wasted work whenever the
                // deps turn out to be stale. The tentative verified-mark
                // above guarantees a recursive verify of `q` couldn't have
                // re-executed it, so the memo entry is still the same one.
                let storage = self.queries.borrow();
                if let Some(memo) = storage.get_memo::<Q>(q) {
                    return (memo.value.clone(), changed_at);
                }
                // Memo unexpectedly gone — fall through to re-execute.
            }

            // Deps changed — fall through to re-execute
        }

        // Phase 2: Execute the query
        self.execute_query(q, qk)
    }

    /// Execute a query from scratch (no valid cache).
    fn execute_query<Q: QueryFn>(&self, q: &Q, qk: &QueryKey) -> (Q::Output, Revision) {
        // Cycle detection
        {
            let stack = self.active.borrow();
            for aq in stack.iter() {
                if aq.key == *qk {
                    eprintln!("=== QUERY CYCLE ===");
                    for (i, s) in stack.iter().enumerate() {
                        eprintln!("  [{}] {}", i, s.label);
                    }
                    let label = q.describe();
                    eprintln!("  --> {}", label);
                    panic!(
                        "Query cycle detected: {} is already on the stack",
                        std::any::type_name::<Q>()
                    );
                }
            }
        }
        self.active.borrow_mut().push(ActiveQuery {
            key: qk.clone(),
            label: q.describe(),
        });

        // Clear old accumulators for this query
        self.accumulators.borrow_mut().clear_for_query(qk);

        // Execute in a fresh dependency scope
        let saved_deps = self.deps.take();
        self.exec_count.set(self.exec_count.get() + 1);
        let result = q.execute(self);
        let new_deps = self.deps.take();
        self.deps.replace(saved_deps);

        // Pop from active stack
        self.active.borrow_mut().pop();

        // Backdating: if the result fingerprint matches the old memo,
        // keep the old changed_at so dependents skip re-execution.
        let new_fp = Fingerprint::of(&result);
        let changed_at = {
            let storage = self.queries.borrow();
            if let Some(old_memo) = storage.get_memo::<Q>(q) {
                if old_memo.fingerprint == new_fp {
                    old_memo.changed_at
                } else {
                    self.revision
                }
            } else {
                self.revision
            }
        };

        // Store memo
        let entry = MemoEntry {
            value: result.clone(),
            fingerprint: new_fp,
            deps: new_deps,
            computed_at: self.revision,
            verified_at: self.revision,
            changed_at,
        };
        self.queries.borrow_mut().insert_memo(q.clone(), entry);

        // Register a type-erased verifier so deps_unchanged can
        // recursively verify this query when it appears as a sub-query dep.
        let q_for_verifier = q.clone();
        let qk_for_verifier = qk.clone();
        self.queries.borrow_mut().register_verifier(
            qk.clone(),
            Arc::new(move |ctx| ctx.ensure_fresh(&q_for_verifier, &qk_for_verifier).1),
        );

        (result, changed_at)
    }

    /// Check if all dependencies are still unchanged.
    ///
    /// For component deps, checks the ChangeSet directly.
    /// For sub-query deps, recursively verifies the sub-query via its
    /// registered type-erased verifier and checks if its changed_at
    /// is still within bounds.
    fn deps_unchanged(&self, deps: &[Dependency], since: Revision) -> bool {
        for dep in deps {
            match dep {
                Dependency::Component { entity, .. } => {
                    if self.changes.is_changed(*entity) {
                        return false;
                    }
                },
                Dependency::Query(sub_qk) => {
                    // Get the verifier for this sub-query (clone Arc, drop borrow)
                    let verifier = {
                        let storage = self.queries.borrow();
                        storage.verifiers.get(sub_qk).cloned()
                    };

                    if let Some(verify) = verifier {
                        // Recursively verify the sub-query. This may trigger
                        // its own deps_unchanged check, or re-execute if its
                        // deps changed.
                        let sub_changed_at = verify(self);
                        if sub_changed_at > since {
                            return false;
                        }
                    } else {
                        // No verifier registered (e.g. after snapshot).
                        // Conservative: assume changed.
                        return false;
                    }
                },
            }
        }
        true
    }
}

/// Synthetic `TypeId` used as the `component_type` field on hierarchy
/// dependency records. `deps_unchanged` only cares whether the entity
/// itself changed (it ignores the type id), so any stable, unique
/// value works — we use the `Hierarchy` struct's TypeId.
fn hierarchy_dep_type() -> TypeId {
    TypeId::of::<crate::world::Hierarchy>()
}

/// Compute a QueryKey for a query value.
fn query_key<Q: QueryFn>(q: &Q) -> QueryKey {
    // Use TypeId bits as the type discriminant
    let type_id_hash = {
        let mut h = FxHasher::default();
        TypeId::of::<Q>().hash(&mut h);
        h.finish()
    };
    let key_hash = {
        let mut h = FxHasher::default();
        q.hash(&mut h);
        h.finish()
    };
    QueryKey {
        type_id: type_id_hash,
        key_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    // -- Test components --
    #[derive(Clone, Debug, PartialEq)]
    struct Name(String);

    #[derive(Clone, Debug, PartialEq)]
    struct Value(i32);

    // -- Test query: read Name component --
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct GetName {
        entity: Entity,
    }

    // Track execution count via thread-local
    thread_local! {
        static EXEC_COUNT: RefCell<u32> = const { RefCell::new(0) };
    }

    fn reset_counter() {
        EXEC_COUNT.with(|c| *c.borrow_mut() = 0);
    }

    fn exec_count() -> u32 {
        EXEC_COUNT.with(|c| *c.borrow())
    }

    impl QueryFn for GetName {
        type Output = Option<String>;

        fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
            EXEC_COUNT.with(|c| *c.borrow_mut() += 1);
            ctx.get::<Name>(self.entity).map(|n| n.0.clone())
        }
    }

    // -- Test query: depends on another query --
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct GetNameUpper {
        entity: Entity,
    }

    impl QueryFn for GetNameUpper {
        type Output = Option<String>;

        fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
            let name = ctx.query(GetName {
                entity: self.entity,
            })?;
            Some(name.to_uppercase())
        }
    }

    // -- Test query: cycle detector --
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct CyclicQuery {
        entity: Entity,
    }

    impl QueryFn for CyclicQuery {
        type Output = i32;

        fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
            // Call self — should panic
            ctx.query(CyclicQuery {
                entity: self.entity,
            })
        }
    }

    #[test]
    fn basic_query_execution() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Alice".into()));

        let ctx = world.query_context();
        let result = ctx.query(GetName { entity: e });
        assert_eq!(result, Some("Alice".into()));
    }

    #[test]
    fn query_memoization() {
        reset_counter();
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Bob".into()));

        let ctx = world.query_context();

        // First call: executes
        let r1 = ctx.query(GetName { entity: e });
        assert_eq!(r1, Some("Bob".into()));
        assert_eq!(exec_count(), 1);

        // Second call same revision: returns cached
        let r2 = ctx.query(GetName { entity: e });
        assert_eq!(r2, Some("Bob".into()));
        assert_eq!(exec_count(), 1); // NOT re-executed
    }

    #[test]
    fn query_reexecutes_after_change() {
        reset_counter();
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Alice".into()));

        // Rev 1: execute
        {
            let ctx = world.query_context();
            ctx.query(GetName { entity: e });
        }
        assert_eq!(exec_count(), 1);

        // Rev 2: change the entity
        world.begin_revision();
        world.set(e, Name("Bob".into()));

        {
            let ctx = world.query_context();
            let result = ctx.query(GetName { entity: e });
            assert_eq!(result, Some("Bob".into()));
        }
        assert_eq!(exec_count(), 2); // re-executed
    }

    #[test]
    fn sub_query_composition() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("alice".into()));

        let ctx = world.query_context();
        let result = ctx.query(GetNameUpper { entity: e });
        assert_eq!(result, Some("ALICE".into()));
    }

    #[test]
    #[should_panic(expected = "Query cycle detected")]
    fn cycle_detection() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();

        let ctx = world.query_context();
        ctx.query(CyclicQuery { entity: e });
    }

    #[test]
    fn accumulator_in_query() {
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct ValidateName {
            entity: Entity,
        }

        impl QueryFn for ValidateName {
            type Output = bool;

            fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
                let name = ctx.get::<Name>(self.entity);
                match name {
                    Some(n) if !n.0.is_empty() => true,
                    _ => {
                        ctx.accumulate("error: name is empty".to_string());
                        false
                    },
                }
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("".into()));

        let ctx = world.query_context();
        let valid = ctx.query(ValidateName { entity: e });
        assert!(!valid);

        let errors = world.accumulated::<String>();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], "error: name is empty");
    }

    #[test]
    fn dependency_recording() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("X".into()));
        world.set(e, Value(42));

        // Query that reads two components
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct ReadBoth {
            entity: Entity,
        }

        impl QueryFn for ReadBoth {
            type Output = String;

            fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
                let name = ctx
                    .get::<Name>(self.entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                let val = ctx.get::<Value>(self.entity).map(|v| v.0).unwrap_or(0);
                format!("{name}={val}")
            }
        }

        let ctx = world.query_context();
        let result = ctx.query(ReadBoth { entity: e });
        assert_eq!(result, "X=42");
    }

    #[test]
    fn sub_query_skips_when_unchanged() {
        // Verify that a query depending on a sub-query is NOT re-executed
        // when the sub-query's inputs haven't changed in a new revision.
        let mut world = World::new();
        world.begin_revision();

        let e1 = world.spawn();
        let e2 = world.spawn();
        world.set(e1, Name("Alice".into()));
        world.set(e2, Name("Bob".into()));

        // Rev 1: execute both GetNameUpper queries
        {
            let ctx = world.query_context();
            ctx.query(GetNameUpper { entity: e1 });
            ctx.query(GetNameUpper { entity: e2 });
        }
        // GetName x2 + GetNameUpper x2 = 4 executions
        assert_eq!(world.query_exec_count(), 4);

        // Rev 2: change only e1
        world.begin_revision();
        world.set(e1, Name("Alicia".into()));

        let before = world.query_exec_count();
        {
            let ctx = world.query_context();
            // e1 changed — both GetName and GetNameUpper must re-execute
            let r1 = ctx.query(GetNameUpper { entity: e1 });
            assert_eq!(r1, Some("ALICIA".into()));

            // e2 unchanged — should be verified from cache, no re-execution
            let r2 = ctx.query(GetNameUpper { entity: e2 });
            assert_eq!(r2, Some("BOB".into()));
        }
        // Only e1's GetName + GetNameUpper = 2 new executions
        assert_eq!(world.query_exec_count() - before, 2);
    }

    #[test]
    fn backdating_skips_downstream_when_value_unchanged() {
        // Tests that backdating (early cutoff) works: if a leaf query
        // re-executes but produces the same value, dependents skip.
        //
        // Setup: GetNameUpper depends on GetName.
        // We change an UNRELATED component on the entity so the entity
        // is marked as changed, forcing GetName to re-execute.
        // But GetName produces the same result → its changed_at is
        // backdated → GetNameUpper should NOT re-execute.
        let mut world = World::new();
        world.begin_revision();

        let e = world.spawn();
        world.set(e, Name("Alice".into()));
        world.set(e, Value(1));

        // Rev 1: execute the query chain (GetName + GetNameUpper = 2)
        {
            let ctx = world.query_context();
            ctx.query(GetNameUpper { entity: e });
        }
        assert_eq!(world.query_exec_count(), 2);

        // Rev 2: change Value (not Name) — entity is marked changed
        world.begin_revision();
        world.set(e, Value(2));

        let before = world.query_exec_count();
        {
            let ctx = world.query_context();
            let result = ctx.query(GetNameUpper { entity: e });
            assert_eq!(result, Some("ALICE".into()));
        }
        // GetName re-executes (entity changed) but produces same result
        // → backdating keeps changed_at at rev1
        // → GetNameUpper sees sub_changed_at <= its verified_at → skips
        let new_execs = world.query_exec_count() - before;
        assert_eq!(new_execs, 1); // Only GetName re-executes, not GetNameUpper
    }

    #[test]
    fn exec_count_tracking() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("X".into()));

        assert_eq!(world.query_exec_count(), 0);

        {
            let ctx = world.query_context();
            ctx.query(GetName { entity: e });
        }
        assert_eq!(world.query_exec_count(), 1);

        // Same revision, cached — no new execution
        {
            let ctx = world.query_context();
            ctx.query(GetName { entity: e });
        }
        assert_eq!(world.query_exec_count(), 1);
    }
}
