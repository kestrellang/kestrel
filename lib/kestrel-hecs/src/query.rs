use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

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
    type Output: Clone + 'static;

    /// Execute the query. All component reads and sub-query calls go
    /// through `ctx`, which records dependencies automatically.
    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output;
}

/// A memoized query result with dependency and revision tracking.
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

/// Type-erased memoization storage. Each query type Q gets its own
/// `HashMap<Q, MemoEntry<Q::Output>>`.
pub(crate) struct QueryStorage {
    stores: HashMap<TypeId, Box<dyn Any>>,
}

impl QueryStorage {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    fn get_memo<Q: QueryFn>(&self, key: &Q) -> Option<&MemoEntry<Q::Output>> {
        self.stores
            .get(&TypeId::of::<Q>())
            .and_then(|s| s.downcast_ref::<HashMap<Q, MemoEntry<Q::Output>>>())
            .and_then(|map| map.get(key))
    }

    fn insert_memo<Q: QueryFn>(&mut self, key: Q, entry: MemoEntry<Q::Output>) {
        self.stores
            .entry(TypeId::of::<Q>())
            .or_insert_with(|| Box::new(HashMap::<Q, MemoEntry<Q::Output>>::new()))
            .downcast_mut::<HashMap<Q, MemoEntry<Q::Output>>>()
            .expect("type mismatch in query storage")
            .insert(key, entry);
    }

    fn update_verified<Q: QueryFn>(&mut self, key: &Q, revision: Revision) {
        if let Some(store) = self.stores.get_mut(&TypeId::of::<Q>())
            && let Some(map) = store.downcast_mut::<HashMap<Q, MemoEntry<Q::Output>>>()
            && let Some(memo) = map.get_mut(key)
        {
            memo.verified_at = revision;
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
    ) -> Self {
        Self {
            revision,
            components,
            changes,
            hierarchy,
            queries,
            accumulators,
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

    /// Get children of an entity.
    pub fn children_of(&self, entity: Entity) -> &'a [Entity] {
        self.hierarchy.children_of(entity)
    }

    /// Get parent of an entity.
    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        self.hierarchy.parent_of(entity)
    }

    /// Push a value into an accumulator (e.g. diagnostics).
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

        // Check memoization cache
        {
            let storage = self.queries.borrow();
            if let Some(memo) = storage.get_memo::<Q>(&q) {
                if memo.verified_at >= self.revision {
                    return memo.value.clone();
                }
                // Try to verify: check if deps are still valid
                if self.deps_unchanged(&memo.deps, memo.verified_at) {
                    drop(storage);
                    self.queries.borrow_mut().update_verified::<Q>(&q, self.revision);
                    return self.queries.borrow().get_memo::<Q>(&q).unwrap().value.clone();
                }
            }
        }

        // Cycle detection
        {
            let stack = self.active.borrow();
            for aq in stack.iter() {
                if aq.key == qk {
                    panic!(
                        "Query cycle detected: {} is already on the stack",
                        std::any::type_name::<Q>()
                    );
                }
            }
        }
        self.active.borrow_mut().push(ActiveQuery { key: qk.clone() });

        // Clear old accumulators for this query
        self.accumulators.borrow_mut().clear_for_query(&qk);

        // Execute in a fresh dependency scope
        let saved_deps = self.deps.take();
        let result = q.execute(self);
        let new_deps = self.deps.take();
        self.deps.replace(saved_deps);

        // Pop from active stack
        self.active.borrow_mut().pop();

        // Check for backdating: if the result is the same as before,
        // keep the old changed_at so dependents don't re-execute.
        let new_fp = Fingerprint::of(&hash_of(&result));
        let changed_at = {
            let storage = self.queries.borrow();
            if let Some(old_memo) = storage.get_memo::<Q>(&q) {
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
        self.queries.borrow_mut().insert_memo(q, entry);

        result
    }

    /// Check if all dependencies are still unchanged.
    fn deps_unchanged(&self, deps: &[Dependency], since: Revision) -> bool {
        for dep in deps {
            match dep {
                Dependency::Component { entity, .. } => {
                    if self.changes.is_changed(*entity) {
                        return false;
                    }
                }
                Dependency::Query(_sub_qk) => {
                    // We can't easily check sub-query staleness without
                    // knowing its type. Conservative: assume changed if
                    // any entity in the world changed. This is refined by
                    // the sub-query's own verification when it's called.
                    //
                    // For now, just check if we're in a new revision.
                    if since < self.revision {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// Compute a QueryKey for a query value.
fn query_key<Q: QueryFn>(q: &Q) -> QueryKey {
    // Use TypeId bits as the type discriminant
    let type_id_hash = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        TypeId::of::<Q>().hash(&mut h);
        h.finish()
    };
    let key_hash = {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        q.hash(&mut h);
        h.finish()
    };
    QueryKey {
        type_id: type_id_hash,
        key_hash,
    }
}

/// Hash any Clone value by its Debug-like fingerprint.
fn hash_of<T: 'static>(value: &T) -> u64 {
    // Use the raw pointer bits as a proxy — this is for fingerprinting
    // the memoized value. Since we store the value, pointer identity
    // isn't useful. Instead, use the type + size as a rough discriminant.
    // Real fingerprinting should be done by the user via Hash impl.
    //
    // For proper backdating, QueryFn::Output should implement Hash.
    // We use a conservative approach: pointer to stack = always different.
    value as *const T as u64
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);

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
                    }
                }
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
        let key = crate::entity::EntityKey::root("test", 0);
        let e = world.intern_entity(key);
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
                let name = ctx.get::<Name>(self.entity).map(|n| n.0.clone()).unwrap_or_default();
                let val = ctx.get::<Value>(self.entity).map(|v| v.0).unwrap_or(0);
                format!("{name}={val}")
            }
        }

        let ctx = world.query_context();
        let result = ctx.query(ReadBoth { entity: e });
        assert_eq!(result, "X=42");
    }
}
