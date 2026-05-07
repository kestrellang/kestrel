use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use crate::accumulator::AccumulatorStore;
use crate::change::ChangeSet;
use crate::component::{Component, ComponentStore};
use crate::entity::Entity;
use crate::query::{QueryContext, QueryStorage};

/// Global revision counter. Incremented each compilation cycle.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Revision(u64);

impl Revision {
    pub fn initial() -> Self {
        Self(0)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

/// Metadata for a single entity. Clone support enables `World::snapshot()`.
#[derive(Clone)]
struct EntityRecord {
    last_changed: Revision,
    alive: bool,
}

/// Parent-child relationships between entities. Clone support enables `World::snapshot()`.
#[derive(Clone)]
pub struct Hierarchy {
    parent: HashMap<Entity, Entity>,
    children: HashMap<Entity, Vec<Entity>>,
}

impl Hierarchy {
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
            children: HashMap::new(),
        }
    }

    pub fn set_parent(&mut self, child: Entity, parent: Entity) {
        // Remove from old parent's children list if re-parenting
        if let Some(&old_parent) = self.parent.get(&child)
            && let Some(siblings) = self.children.get_mut(&old_parent)
        {
            siblings.retain(|&e| e != child);
        }
        self.parent.insert(child, parent);
        self.children.entry(parent).or_default().push(child);
    }

    /// Detach `entity` from the hierarchy: drop its parent link, remove
    /// it from its parent's children list, and forget its own children
    /// list. Returns the former parent so the caller can mark it as
    /// changed (its children list mutated). Children become orphans —
    /// callers should despawn them too.
    pub fn detach(&mut self, entity: Entity) -> Option<Entity> {
        let parent = self.parent.remove(&entity);
        if let Some(p) = parent
            && let Some(siblings) = self.children.get_mut(&p)
        {
            siblings.retain(|&e| e != entity);
        }
        self.children.remove(&entity);
        parent
    }

    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        self.parent.get(&entity).copied()
    }

    pub fn children_of(&self, entity: Entity) -> &[Entity] {
        self.children
            .get(&entity)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Walk ancestors from entity to root (inclusive of entity).
    pub fn ancestors(&self, entity: Entity) -> AncestorIter<'_> {
        AncestorIter {
            hierarchy: self,
            current: Some(entity),
        }
    }
}

impl Default for Hierarchy {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AncestorIter<'a> {
    hierarchy: &'a Hierarchy,
    current: Option<Entity>,
}

impl Iterator for AncestorIter<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let current = self.current?;
        self.current = self.hierarchy.parent_of(current);
        Some(current)
    }
}

/// Central compilation database. Persists across compilations.
///
/// Owns all entities, components, hierarchy, change tracking, query
/// caches, and accumulators. Use mutation methods (set, set_parent, etc.)
/// during mutation phases, then `query_context()` for query phases.
pub struct World {
    revision: Revision,
    entities: Vec<EntityRecord>,
    components: ComponentStore,
    hierarchy: Hierarchy,
    changes: ChangeSet,
    queries: RefCell<QueryStorage>,
    accumulators: RefCell<AccumulatorStore>,
    /// Counter for actual query executions (not cache hits).
    /// Incremented by QueryContext::execute_query().
    query_exec_count: Cell<u64>,
}

impl World {
    pub fn new() -> Self {
        Self {
            revision: Revision::initial(),
            entities: Vec::new(),
            components: ComponentStore::new(),
            hierarchy: Hierarchy::new(),
            changes: ChangeSet::new(),
            queries: RefCell::new(QueryStorage::new()),
            accumulators: RefCell::new(AccumulatorStore::new()),
            query_exec_count: Cell::new(0),
        }
    }

    /// Begin a new compilation cycle. Advances the revision and
    /// prepares change tracking.
    pub fn begin_revision(&mut self) -> Revision {
        self.revision = self.revision.next();
        self.changes.advance();
        self.revision
    }

    pub fn revision(&self) -> Revision {
        self.revision
    }

    /// Allocate a fresh entity. Returns a unique handle.
    pub fn spawn(&mut self) -> Entity {
        let entity = Entity::from_raw(self.entities.len() as u32);
        self.entities.push(EntityRecord {
            last_changed: self.revision,
            alive: true,
        });
        self.changes.mark_changed(entity);
        entity
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        entity.index() < self.entities.len() && self.entities[entity.index()].alive
    }

    /// Mark `entity` as dead and remove every component it carries.
    /// Detaches it from the hierarchy and marks it as changed so any
    /// memoized query that read its components re-fires on next access.
    ///
    /// Entity IDs are not reused — `spawn()` always allocates a fresh
    /// index, so a despawned entity stays a stable "tombstone" that
    /// reports `is_alive == false` and `get == None` forever.
    pub fn despawn(&mut self, entity: Entity) {
        if !self.is_alive(entity) {
            return;
        }
        self.components.despawn_all(entity);
        let former_parent = self.hierarchy.detach(entity);
        self.entities[entity.index()].alive = false;
        self.entities[entity.index()].last_changed = self.revision;
        self.changes.mark_changed(entity);
        // Detaching mutated the parent's children list — mark it
        // changed so queries that walked `children_of(parent)` see
        // the despawned entity disappear from cache.
        if let Some(p) = former_parent {
            self.changes.mark_changed(p);
            self.entities[p.index()].last_changed = self.revision;
        }
    }

    /// When this entity was last modified.
    pub fn last_changed(&self, entity: Entity) -> Revision {
        self.entities[entity.index()].last_changed
    }

    /// Total number of entities (including dead).
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // -- Component access (mutation phase) --

    /// Attach a component to an entity. Marks the entity as changed.
    pub fn set<T: Component>(&mut self, entity: Entity, value: T) {
        self.components.insert(entity, value);
        self.entities[entity.index()].last_changed = self.revision;
        self.changes.mark_changed(entity);
    }

    /// Get a component from an entity (during mutation phase).
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.components.get::<T>(entity)
    }

    /// Get a mutable component reference.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.entities[entity.index()].last_changed = self.revision;
        self.changes.mark_changed(entity);
        self.components.get_mut::<T>(entity)
    }

    /// Check if an entity has a component.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.components.has::<T>(entity)
    }

    /// Remove a component from an entity.
    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> bool {
        if self.components.remove::<T>(entity) {
            self.entities[entity.index()].last_changed = self.revision;
            self.changes.mark_changed(entity);
            true
        } else {
            false
        }
    }

    /// Iterate over all (entity, component) pairs for a type.
    pub fn iter_component<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.components.iter::<T>()
    }

    // -- Hierarchy --

    pub fn set_parent(&mut self, child: Entity, parent: Entity) {
        // Snapshot the old parent before mutation so we can mark it
        // changed too — moving a child away from `old_parent` mutates
        // its children list, and queries that read it must re-fire.
        let old_parent = self.hierarchy.parent_of(child);
        self.hierarchy.set_parent(child, parent);
        // The parent's children list changed; queries that walked it
        // (via `QueryContext::children_of`) need to re-fire.
        self.changes.mark_changed(parent);
        self.entities[parent.index()].last_changed = self.revision;
        if let Some(op) = old_parent
            && op != parent
        {
            self.changes.mark_changed(op);
            self.entities[op.index()].last_changed = self.revision;
        }
    }

    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        self.hierarchy.parent_of(entity)
    }

    pub fn children_of(&self, entity: Entity) -> &[Entity] {
        self.hierarchy.children_of(entity)
    }

    pub fn ancestors(&self, entity: Entity) -> AncestorIter<'_> {
        self.hierarchy.ancestors(entity)
    }

    // -- Change tracking --

    pub fn changes(&self) -> &ChangeSet {
        &self.changes
    }

    pub fn changes_mut(&mut self) -> &mut ChangeSet {
        &mut self.changes
    }

    // -- Query phase --

    /// Create a query context for the read phase.
    ///
    /// During the query phase, the world is borrowed immutably.
    /// All reads go through the QueryContext which records dependencies.
    pub fn query_context(&self) -> QueryContext<'_> {
        QueryContext::new(
            self.revision,
            &self.components,
            &self.changes,
            &self.hierarchy,
            &self.queries,
            &self.accumulators,
            &self.query_exec_count,
        )
    }

    /// Number of actual query executions since the world was created.
    /// Does not count cache hits or verifications — only full re-executions.
    pub fn query_exec_count(&self) -> u64 {
        self.query_exec_count.get()
    }

    // -- Snapshot --

    /// Create a structural clone of this world for reuse across compilations.
    ///
    /// Clones entities, components, hierarchy, and change tracking. Starts
    /// with fresh query caches and accumulators — they rebuild lazily on
    /// first access. Keeps the same revision so `begin_revision()` works
    /// normally on the snapshot.
    pub fn snapshot(&self) -> World {
        World {
            revision: self.revision,
            entities: self.entities.clone(),
            components: self.components.clone(),
            hierarchy: self.hierarchy.clone(),
            changes: self.changes.clone(),
            queries: RefCell::new(self.queries.borrow().clone()),
            accumulators: RefCell::new(AccumulatorStore::new()),
            query_exec_count: Cell::new(0),
        }
    }

    // -- Accumulators --

    /// Collect all accumulated values of type T into a Vec.
    pub fn accumulated<T: Clone + 'static>(&self) -> Vec<T> {
        self.accumulators.borrow().all::<T>().cloned().collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Name(String);

    #[derive(Clone, Debug, PartialEq)]
    struct Health(i32);

    #[test]
    fn set_and_get_components() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Alice".into()));
        world.set(e, Health(100));

        assert_eq!(world.get::<Name>(e), Some(&Name("Alice".into())));
        assert_eq!(world.get::<Health>(e), Some(&Health(100)));
    }

    #[test]
    fn hierarchy() {
        let mut world = World::new();
        let parent = world.spawn();
        let child1 = world.spawn();
        let child2 = world.spawn();

        world.set_parent(child1, parent);
        world.set_parent(child2, parent);

        assert_eq!(world.parent_of(child1), Some(parent));
        assert_eq!(world.parent_of(child2), Some(parent));
        assert_eq!(world.children_of(parent).len(), 2);
        assert!(world.children_of(parent).contains(&child1));
        assert!(world.children_of(parent).contains(&child2));
    }

    #[test]
    fn ancestor_walk() {
        let mut world = World::new();
        let root = world.spawn();
        let mid = world.spawn();
        let leaf = world.spawn();

        world.set_parent(mid, root);
        world.set_parent(leaf, mid);

        let ancestors: Vec<_> = world.ancestors(leaf).collect();
        assert_eq!(ancestors, vec![leaf, mid, root]);
    }

    #[test]
    fn reparent() {
        let mut world = World::new();
        let p1 = world.spawn();
        let p2 = world.spawn();
        let child = world.spawn();

        world.set_parent(child, p1);
        assert_eq!(world.children_of(p1).len(), 1);

        // Re-parent
        world.set_parent(child, p2);
        assert_eq!(world.parent_of(child), Some(p2));
        assert_eq!(world.children_of(p1).len(), 0);
        assert_eq!(world.children_of(p2).len(), 1);
    }

    #[test]
    fn revision_lifecycle() {
        let mut world = World::new();
        assert_eq!(world.revision(), Revision::initial());

        let r1 = world.begin_revision();
        assert_eq!(r1.value(), 1);

        let r2 = world.begin_revision();
        assert_eq!(r2.value(), 2);
    }

    #[test]
    fn set_marks_entity_changed() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        assert!(world.changes().is_changed(e)); // spawn marks changed

        world.begin_revision(); // clears changes
        assert!(!world.changes().is_changed(e));

        world.set(e, Name("X".into()));
        assert!(world.changes().is_changed(e));
    }

    #[test]
    fn remove_component() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(100));
        assert!(world.has::<Health>(e));

        world.remove_component::<Health>(e);
        assert!(!world.has::<Health>(e));
    }

    #[test]
    fn iter_components() {
        let mut world = World::new();
        world.begin_revision();

        let e1 = world.spawn();
        let e2 = world.spawn();

        world.set(e1, Health(10));
        world.set(e2, Health(20));

        let healths: Vec<_> = world.iter_component::<Health>().collect();
        assert_eq!(healths.len(), 2);
    }

    // -- Snapshot tests --

    #[test]
    fn snapshot_preserves_entities_and_components() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Alice".into()));
        world.set(e, Health(100));

        let snap = world.snapshot();
        assert_eq!(snap.entity_count(), 1);
        assert_eq!(snap.get::<Name>(e), Some(&Name("Alice".into())));
        assert_eq!(snap.get::<Health>(e), Some(&Health(100)));
        assert!(snap.is_alive(e));
    }

    #[test]
    fn snapshot_preserves_hierarchy() {
        let mut world = World::new();
        world.begin_revision();
        let parent = world.spawn();
        let child = world.spawn();
        world.set_parent(child, parent);

        let snap = world.snapshot();
        assert_eq!(snap.parent_of(child), Some(parent));
        assert_eq!(snap.children_of(parent), &[child]);
    }

    #[test]
    fn snapshot_isolates_mutations() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(100));

        let mut snap = world.snapshot();

        // Mutate the snapshot
        snap.begin_revision();
        snap.set(e, Health(999));
        let e2 = snap.spawn();
        snap.set(e2, Name("Snapshot-only".into()));

        // Original is unaffected
        assert_eq!(world.get::<Health>(e), Some(&Health(100)));
        assert_eq!(world.entity_count(), 1);

        // Snapshot has the changes
        assert_eq!(snap.get::<Health>(e), Some(&Health(999)));
        assert_eq!(snap.entity_count(), 2);
    }

    #[test]
    fn snapshot_preserves_query_cache() {
        use crate::query::QueryFn;
        use std::cell::RefCell;

        thread_local! {
            static SNAP_EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct GetHealth {
            entity: Entity,
        }

        impl QueryFn for GetHealth {
            type Output = Option<i32>;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                SNAP_EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.get::<Health>(self.entity).map(|h| h.0)
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(42));

        // Execute query on original — populates cache
        {
            let ctx = world.query_context();
            ctx.query(GetHealth { entity: e });
        }
        SNAP_EXEC.with(|c| {
            c.replace(0);
        });

        // begin_revision clears the changed set; cached queries whose deps
        // are unchanged will be verified without re-execution.
        world.begin_revision();

        let snap = world.snapshot();
        let ctx = snap.query_context();

        // Cached query survives snapshot — entity is NOT in the changed set,
        // so deps_unchanged returns true and we get a cache hit.
        let result = ctx.query(GetHealth { entity: e });
        assert_eq!(result, Some(42));
        assert_eq!(SNAP_EXEC.with(|c| *c.borrow()), 0);

        // Second call is still memoized
        let result2 = ctx.query(GetHealth { entity: e });
        assert_eq!(result2, Some(42));
        assert_eq!(SNAP_EXEC.with(|c| *c.borrow()), 0);
    }

    #[test]
    fn snapshot_invalidates_changed_deps() {
        use crate::query::QueryFn;
        use std::cell::RefCell;

        thread_local! {
            static INV_EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct GetHealth {
            entity: Entity,
        }

        impl QueryFn for GetHealth {
            type Output = Option<i32>;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                INV_EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.get::<Health>(self.entity).map(|h| h.0)
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(42));

        // Populate cache
        {
            let ctx = world.query_context();
            ctx.query(GetHealth { entity: e });
        }
        INV_EXEC.with(|c| { c.replace(0); });

        // Mutate the entity in a new revision — it's now in the changed set
        world.begin_revision();
        world.set(e, Health(99));

        let snap = world.snapshot();
        let ctx = snap.query_context();

        // Cached query is invalidated because entity is changed; re-executes
        let result = ctx.query(GetHealth { entity: e });
        assert_eq!(result, Some(99));
        assert_eq!(INV_EXEC.with(|c| *c.borrow()), 1);
    }

    // -- Despawn tests --

    #[test]
    fn despawn_then_read_returns_none() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Name("Alice".into()));
        world.set(e, Health(100));

        world.despawn(e);

        assert!(!world.is_alive(e));
        assert_eq!(world.get::<Name>(e), None);
        assert_eq!(world.get::<Health>(e), None);
        assert!(!world.has::<Name>(e));
    }

    #[test]
    fn despawn_skips_entity_in_iter_component() {
        let mut world = World::new();
        world.begin_revision();
        let e1 = world.spawn();
        let e2 = world.spawn();
        world.set(e1, Health(10));
        world.set(e2, Health(20));

        world.despawn(e1);

        let healths: Vec<_> = world.iter_component::<Health>().collect();
        assert_eq!(healths.len(), 1);
        assert_eq!(healths[0].0, e2);
    }

    #[test]
    fn despawn_detaches_from_parent() {
        let mut world = World::new();
        world.begin_revision();
        let parent = world.spawn();
        let child = world.spawn();
        world.set_parent(child, parent);
        assert_eq!(world.children_of(parent), &[child]);

        world.despawn(child);

        assert_eq!(world.parent_of(child), None);
        assert!(world.children_of(parent).is_empty());
    }

    #[test]
    fn respawn_after_despawn_yields_new_id() {
        let mut world = World::new();
        world.begin_revision();
        let e1 = world.spawn();
        world.despawn(e1);
        let e2 = world.spawn();
        assert_ne!(e1, e2);
        assert!(!world.is_alive(e1));
        assert!(world.is_alive(e2));
    }

    #[test]
    fn despawn_marks_entity_changed() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(100));

        world.begin_revision(); // clear changes
        assert!(!world.changes().is_changed(e));

        world.despawn(e);
        assert!(world.changes().is_changed(e));
    }

    #[test]
    fn despawn_invalidates_dependent_query() {
        use crate::query::QueryFn;
        use std::cell::RefCell;

        thread_local! {
            static EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct GetHealth {
            entity: Entity,
        }

        impl QueryFn for GetHealth {
            type Output = Option<i32>;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.get::<Health>(self.entity).map(|h| h.0)
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(42));

        // First call: executes, caches.
        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(GetHealth { entity: e }), Some(42));
        }
        assert_eq!(EXEC.with(|c| *c.borrow()), 1);

        // Despawn between revisions, then query again — should re-execute
        // because the entity's component dep is marked changed and the
        // read now returns None.
        world.begin_revision();
        world.despawn(e);

        let ctx = world.query_context();
        let r2 = ctx.query(GetHealth { entity: e });
        assert_eq!(r2, None);
        assert_eq!(EXEC.with(|c| *c.borrow()), 2);
    }

    #[test]
    fn despawn_invalidates_children_of_query() {
        // A query that walks `ctx.children_of(parent)` and reads a
        // component on each child must re-fire when one of those
        // children is despawned, even though it never read a
        // component on the parent itself. Before the hierarchy-dep
        // tracking fix, the query would return stale results
        // pointing at dead entities.
        use crate::query::QueryFn;
        use std::cell::RefCell;

        thread_local! {
            static EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct ChildNames {
            parent: Entity,
        }

        impl QueryFn for ChildNames {
            type Output = Vec<String>;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.children_of(self.parent)
                    .iter()
                    .filter_map(|&c| ctx.get::<Name>(c).map(|n| n.0.clone()))
                    .collect()
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let parent = world.spawn();
        let c1 = world.spawn();
        world.set(c1, Name("alpha".into()));
        world.set_parent(c1, parent);
        let c2 = world.spawn();
        world.set(c2, Name("beta".into()));
        world.set_parent(c2, parent);

        // First execution.
        {
            let ctx = world.query_context();
            let mut names = ctx.query(ChildNames { parent });
            names.sort();
            assert_eq!(names, vec!["alpha".to_string(), "beta".to_string()]);
        }
        assert_eq!(EXEC.with(|c| *c.borrow()), 1);

        // Despawn one child between revisions.
        world.begin_revision();
        world.despawn(c1);

        // Query must re-fire and see only the surviving child.
        let ctx = world.query_context();
        let names = ctx.query(ChildNames { parent });
        assert_eq!(names, vec!["beta".to_string()]);
        assert_eq!(EXEC.with(|c| *c.borrow()), 2);
    }

    #[test]
    fn set_parent_invalidates_children_of_query() {
        // Adding a new child to a parent must invalidate cached
        // queries that read the parent's children list.
        use crate::query::QueryFn;
        use std::cell::RefCell;

        thread_local! {
            static EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct ChildCount {
            parent: Entity,
        }

        impl QueryFn for ChildCount {
            type Output = usize;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.children_of(self.parent).len()
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let parent = world.spawn();

        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(ChildCount { parent }), 0);
        }
        assert_eq!(EXEC.with(|c| *c.borrow()), 1);

        world.begin_revision();
        let new_child = world.spawn();
        world.set_parent(new_child, parent);

        let ctx = world.query_context();
        assert_eq!(ctx.query(ChildCount { parent }), 1);
        assert_eq!(EXEC.with(|c| *c.borrow()), 2);
    }

    #[test]
    fn despawn_idempotent() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        world.set(e, Health(100));
        world.despawn(e);
        // Calling again must not panic.
        world.despawn(e);
        assert!(!world.is_alive(e));
    }

    #[test]
    fn snapshot_preserves_revision() {
        let mut world = World::new();
        world.begin_revision();
        world.begin_revision();
        assert_eq!(world.revision().value(), 2);

        let mut snap = world.snapshot();
        assert_eq!(snap.revision().value(), 2);

        // Can advance normally
        let r = snap.begin_revision();
        assert_eq!(r.value(), 3);

        // Original unchanged
        assert_eq!(world.revision().value(), 2);
    }
}
