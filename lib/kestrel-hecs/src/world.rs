use std::cell::RefCell;
use std::collections::HashMap;

use crate::accumulator::AccumulatorStore;
use crate::change::ChangeSet;
use crate::component::{Component, ComponentStore};
use crate::entity::{Entity, EntityKey};
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
    key: EntityKey,
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
    key_to_entity: HashMap<EntityKey, Entity>,
    components: ComponentStore,
    hierarchy: Hierarchy,
    changes: ChangeSet,
    queries: RefCell<QueryStorage>,
    accumulators: RefCell<AccumulatorStore>,
}

impl World {
    pub fn new() -> Self {
        Self {
            revision: Revision::initial(),
            entities: Vec::new(),
            key_to_entity: HashMap::new(),
            components: ComponentStore::new(),
            hierarchy: Hierarchy::new(),
            changes: ChangeSet::new(),
            queries: RefCell::new(QueryStorage::new()),
            accumulators: RefCell::new(AccumulatorStore::new()),
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

    /// Intern an entity key, returning its handle.
    ///
    /// If the key was seen in a previous compilation, returns the SAME
    /// Entity handle — this is how cross-compilation identity works.
    pub fn intern_entity(&mut self, key: EntityKey) -> Entity {
        if let Some(&entity) = self.key_to_entity.get(&key) {
            return entity;
        }
        let entity = Entity::from_raw(self.entities.len() as u32);
        self.entities.push(EntityRecord {
            key: key.clone(),
            last_changed: self.revision,
            alive: true,
        });
        self.key_to_entity.insert(key, entity);
        // New entities are inherently changed
        self.changes.mark_changed(entity);
        entity
    }

    /// Get the stable key for an entity.
    pub fn entity_key(&self, entity: Entity) -> &EntityKey {
        &self.entities[entity.index()].key
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        entity.index() < self.entities.len() && self.entities[entity.index()].alive
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
        self.hierarchy.set_parent(child, parent);
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
        )
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
            key_to_entity: self.key_to_entity.clone(),
            components: self.components.clone(),
            hierarchy: self.hierarchy.clone(),
            changes: self.changes.clone(),
            queries: RefCell::new(QueryStorage::new()),
            accumulators: RefCell::new(AccumulatorStore::new()),
        }
    }

    // -- Accumulators --

    /// Collect all accumulated values of type T into a Vec.
    pub fn accumulated<T: Clone + 'static>(&self) -> Vec<T> {
        self.accumulators
            .borrow()
            .all::<T>()
            .cloned()
            .collect()
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
    fn intern_entity_stable() {
        let mut world = World::new();
        let key = EntityKey::root("test", 0);
        let e1 = world.intern_entity(key.clone());
        let e2 = world.intern_entity(key);
        assert_eq!(e1, e2);
    }

    #[test]
    fn intern_different_keys() {
        let mut world = World::new();
        let e1 = world.intern_entity(EntityKey::root("a", 0));
        let e2 = world.intern_entity(EntityKey::root("b", 0));
        assert_ne!(e1, e2);
    }

    #[test]
    fn set_and_get_components() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.intern_entity(EntityKey::root("test", 0));
        world.set(e, Name("Alice".into()));
        world.set(e, Health(100));

        assert_eq!(world.get::<Name>(e), Some(&Name("Alice".into())));
        assert_eq!(world.get::<Health>(e), Some(&Health(100)));
    }

    #[test]
    fn hierarchy() {
        let mut world = World::new();
        let parent = world.intern_entity(EntityKey::root("parent", 0));
        let child1 = world.intern_entity(EntityKey::root("parent", 0).child("c1", 1));
        let child2 = world.intern_entity(EntityKey::root("parent", 0).child("c2", 1));

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
        let root = world.intern_entity(EntityKey::root("root", 0));
        let mid = world.intern_entity(EntityKey::root("root", 0).child("mid", 0));
        let leaf = world.intern_entity(EntityKey::root("root", 0).child("mid", 0).child("leaf", 0));

        world.set_parent(mid, root);
        world.set_parent(leaf, mid);

        let ancestors: Vec<_> = world.ancestors(leaf).collect();
        assert_eq!(ancestors, vec![leaf, mid, root]);
    }

    #[test]
    fn reparent() {
        let mut world = World::new();
        let p1 = world.intern_entity(EntityKey::root("p1", 0));
        let p2 = world.intern_entity(EntityKey::root("p2", 0));
        let child = world.intern_entity(EntityKey::root("child", 0));

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
        let e = world.intern_entity(EntityKey::root("test", 0));
        assert!(world.changes().is_changed(e)); // intern marks changed

        world.begin_revision(); // clears changes
        assert!(!world.changes().is_changed(e));

        world.set(e, Name("X".into()));
        assert!(world.changes().is_changed(e));
    }

    #[test]
    fn remove_component() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.intern_entity(EntityKey::root("test", 0));
        world.set(e, Health(100));
        assert!(world.has::<Health>(e));

        world.remove_component::<Health>(e);
        assert!(!world.has::<Health>(e));
    }

    #[test]
    fn entity_key_roundtrip() {
        let mut world = World::new();
        let key = EntityKey::root("main.ks", 0);
        let e = world.intern_entity(key.clone());
        assert_eq!(world.entity_key(e), &key);
    }

    #[test]
    fn iter_components() {
        let mut world = World::new();
        world.begin_revision();

        let e1 = world.intern_entity(EntityKey::root("a", 0));
        let e2 = world.intern_entity(EntityKey::root("b", 0));

        world.set(e1, Health(10));
        world.set(e2, Health(20));

        let healths: Vec<_> = world.iter_component::<Health>().collect();
        assert_eq!(healths.len(), 2);
    }

    #[test]
    fn intern_survives_across_revisions() {
        let mut world = World::new();
        let key = EntityKey::root("persistent", 0);

        world.begin_revision();
        let e1 = world.intern_entity(key.clone());
        world.set(e1, Name("v1".into()));

        world.begin_revision();
        let e2 = world.intern_entity(key);
        assert_eq!(e1, e2); // same handle
        assert_eq!(world.get::<Name>(e2), Some(&Name("v1".into()))); // data persists
    }

    // -- Snapshot tests --

    #[test]
    fn snapshot_preserves_entities_and_components() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.intern_entity(EntityKey::root("test", 0));
        world.set(e, Name("Alice".into()));
        world.set(e, Health(100));

        let snap = world.snapshot();
        assert_eq!(snap.entity_count(), 1);
        assert_eq!(snap.get::<Name>(e), Some(&Name("Alice".into())));
        assert_eq!(snap.get::<Health>(e), Some(&Health(100)));
        assert!(snap.is_alive(e));
        assert_eq!(snap.entity_key(e), &EntityKey::root("test", 0));
    }

    #[test]
    fn snapshot_preserves_hierarchy() {
        let mut world = World::new();
        world.begin_revision();
        let parent = world.intern_entity(EntityKey::root("parent", 0));
        let child = world.intern_entity(EntityKey::root("parent", 0).child("c", 1));
        world.set_parent(child, parent);

        let snap = world.snapshot();
        assert_eq!(snap.parent_of(child), Some(parent));
        assert_eq!(snap.children_of(parent), &[child]);
    }

    #[test]
    fn snapshot_isolates_mutations() {
        let mut world = World::new();
        world.begin_revision();
        let e = world.intern_entity(EntityKey::root("test", 0));
        world.set(e, Health(100));

        let mut snap = world.snapshot();

        // Mutate the snapshot
        snap.begin_revision();
        snap.set(e, Health(999));
        let e2 = snap.intern_entity(EntityKey::root("new", 0));
        snap.set(e2, Name("Snapshot-only".into()));

        // Original is unaffected
        assert_eq!(world.get::<Health>(e), Some(&Health(100)));
        assert_eq!(world.entity_count(), 1);

        // Snapshot has the changes
        assert_eq!(snap.get::<Health>(e), Some(&Health(999)));
        assert_eq!(snap.entity_count(), 2);
    }

    #[test]
    fn snapshot_has_fresh_query_cache() {
        use std::cell::RefCell;
        use crate::query::QueryFn;

        thread_local! {
            static SNAP_EXEC: RefCell<u32> = const { RefCell::new(0) };
        }

        #[derive(Clone, PartialEq, Eq, Hash)]
        struct GetHealth { entity: Entity }

        impl QueryFn for GetHealth {
            type Output = Option<i32>;
            fn execute(&self, ctx: &crate::query::QueryContext<'_>) -> Self::Output {
                SNAP_EXEC.with(|c| *c.borrow_mut() += 1);
                ctx.get::<Health>(self.entity).map(|h| h.0)
            }
        }

        let mut world = World::new();
        world.begin_revision();
        let e = world.intern_entity(EntityKey::root("test", 0));
        world.set(e, Health(42));

        // Execute query on original — populates cache
        {
            let ctx = world.query_context();
            ctx.query(GetHealth { entity: e });
        }
        SNAP_EXEC.with(|c| { c.replace(0); });

        // Snapshot starts with empty cache, so query must re-execute
        let snap = world.snapshot();
        let ctx = snap.query_context();
        let result = ctx.query(GetHealth { entity: e });
        assert_eq!(result, Some(42));
        assert_eq!(SNAP_EXEC.with(|c| *c.borrow()), 1);

        // Second call on snapshot is memoized
        let result2 = ctx.query(GetHealth { entity: e });
        assert_eq!(result2, Some(42));
        assert_eq!(SNAP_EXEC.with(|c| *c.borrow()), 1);
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
