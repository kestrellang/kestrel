use std::any::{Any, TypeId};

use rustc_hash::FxHashMap;

use crate::entity::Entity;

/// Trait alias for types that can be stored as components.
///
/// Blanket-implemented for all `Any + Clone + 'static` types, so users
/// never need to implement this manually — just use any cloneable type.
pub trait Component: Any + Clone + 'static {}

impl<T: Any + Clone + 'static> Component for T {}

// -- Type-erased column interface --

/// Type-erased operations on a component column.
#[allow(dead_code)]
trait AnyColumn: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn has(&self, entity: Entity) -> bool;
    fn remove(&mut self, entity: Entity) -> bool;
    fn len(&self) -> usize;
    /// Deep-clone this column into a new box. Possible because T: Component requires Clone.
    fn clone_box(&self) -> Box<dyn AnyColumn>;
}

/// Dense column storage for a single component type.
///
/// Uses a sparse-dense pattern: `entity_to_index` maps Entity -> index
/// in the `dense` vec, giving O(1) lookup. The dense vec enables
/// cache-friendly iteration over all entities with this component.
struct TypedColumn<T> {
    entity_to_index: FxHashMap<Entity, usize>,
    dense: Vec<(Entity, T)>,
}

impl<T: Component> TypedColumn<T> {
    fn new() -> Self {
        Self {
            entity_to_index: FxHashMap::default(),
            dense: Vec::new(),
        }
    }

    fn insert(&mut self, entity: Entity, value: T) {
        if let Some(&idx) = self.entity_to_index.get(&entity) {
            self.dense[idx].1 = value;
        } else {
            let idx = self.dense.len();
            self.entity_to_index.insert(entity, idx);
            self.dense.push((entity, value));
        }
    }

    fn get(&self, entity: Entity) -> Option<&T> {
        self.entity_to_index
            .get(&entity)
            .map(|&idx| &self.dense[idx].1)
    }

    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.entity_to_index
            .get(&entity)
            .map(|&idx| &mut self.dense[idx].1)
    }

    fn has(&self, entity: Entity) -> bool {
        self.entity_to_index.contains_key(&entity)
    }

    /// Remove a component. Uses swap-remove to maintain dense packing.
    fn remove(&mut self, entity: Entity) -> bool {
        let Some(idx) = self.entity_to_index.remove(&entity) else {
            return false;
        };
        let last = self.dense.len() - 1;
        if idx != last {
            // Swap the removed slot with the last element
            self.dense.swap(idx, last);
            // Update the swapped entity's index
            let swapped_entity = self.dense[idx].0;
            self.entity_to_index.insert(swapped_entity, idx);
        }
        self.dense.pop();
        true
    }

    fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.dense.iter().map(|(e, v)| (*e, v))
    }

    fn len(&self) -> usize {
        self.dense.len()
    }
}

impl<T: Component> AnyColumn for TypedColumn<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn has(&self, entity: Entity) -> bool {
        self.has(entity)
    }
    fn remove(&mut self, entity: Entity) -> bool {
        self.remove(entity)
    }
    fn len(&self) -> usize {
        self.len()
    }
    fn clone_box(&self) -> Box<dyn AnyColumn> {
        Box::new(TypedColumn {
            entity_to_index: self.entity_to_index.clone(),
            dense: self.dense.clone(),
        })
    }
}

/// Storage for all component columns, keyed by component TypeId.
///
/// Replaces the current `Vec<Arc<dyn Behavior>>` on SymbolMetadata.
/// Each component type T gets its own dense column, giving type-safe
/// access and cache-friendly iteration.
pub struct ComponentStore {
    columns: FxHashMap<TypeId, Box<dyn AnyColumn>>,
}

impl ComponentStore {
    pub fn new() -> Self {
        Self {
            columns: FxHashMap::default(),
        }
    }

    /// Attach a component to an entity. Overwrites if already present.
    pub fn insert<T: Component>(&mut self, entity: Entity, value: T) {
        self.column_mut::<T>().insert(entity, value);
    }

    /// Get a component for an entity.
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.column::<T>()?.get(entity)
    }

    /// Get a mutable reference to a component.
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.column_mut::<T>().get_mut(entity)
    }

    /// Check if an entity has a component.
    pub fn has<T: Component>(&self, entity: Entity) -> bool {
        self.column::<T>().is_some_and(|col| col.has(entity))
    }

    /// Remove a component from an entity. Returns true if it was present.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> bool {
        if self.column::<T>().is_some() {
            self.column_mut::<T>().remove(entity)
        } else {
            false
        }
    }

    /// Iterate over all (entity, component) pairs for a given type.
    pub fn iter<T: Component>(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.column::<T>().into_iter().flat_map(|col| col.iter())
    }

    /// Count of entities with a given component type.
    pub fn count<T: Component>(&self) -> usize {
        self.column::<T>().map_or(0, |col| col.len())
    }

    /// Remove this entity from every column it appears in. After this
    /// call `get` returns `None` and `iter` skips the entity for every
    /// component type. Used by `World::despawn`.
    pub fn despawn_all(&mut self, entity: Entity) {
        for col in self.columns.values_mut() {
            col.remove(entity);
        }
    }

    // -- private helpers --

    fn column<T: Component>(&self) -> Option<&TypedColumn<T>> {
        self.columns
            .get(&TypeId::of::<T>())
            .and_then(|col| col.as_any().downcast_ref::<TypedColumn<T>>())
    }

    fn column_mut<T: Component>(&mut self) -> &mut TypedColumn<T> {
        self.columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedColumn::<T>::new()))
            .as_any_mut()
            .downcast_mut::<TypedColumn<T>>()
            .expect("type mismatch in component store")
    }
}

/// Manual Clone impl because `Box<dyn AnyColumn>` isn't Clone.
/// Each column clones itself via `clone_box()`, which works because
/// `T: Component` requires `Clone`. Used by `World::snapshot()`.
impl Clone for ComponentStore {
    fn clone(&self) -> Self {
        Self {
            columns: self
                .columns
                .iter()
                .map(|(&k, v)| (k, v.clone_box()))
                .collect(),
        }
    }
}

impl Default for ComponentStore {
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

    #[derive(Clone, Debug, PartialEq)]
    struct Marker;

    fn e(n: u32) -> Entity {
        Entity::from_raw(n)
    }

    #[test]
    fn insert_and_get() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Name("Alice".into()));
        store.insert(e(1), Name("Bob".into()));

        assert_eq!(store.get::<Name>(e(0)), Some(&Name("Alice".into())));
        assert_eq!(store.get::<Name>(e(1)), Some(&Name("Bob".into())));
        assert_eq!(store.get::<Name>(e(2)), None);
    }

    #[test]
    fn overwrite() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Health(100));
        store.insert(e(0), Health(50));
        assert_eq!(store.get::<Health>(e(0)), Some(&Health(50)));
    }

    #[test]
    fn has_component() {
        let mut store = ComponentStore::new();
        assert!(!store.has::<Name>(e(0)));
        store.insert(e(0), Name("X".into()));
        assert!(store.has::<Name>(e(0)));
        assert!(!store.has::<Health>(e(0)));
    }

    #[test]
    fn remove_component() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Name("A".into()));
        store.insert(e(1), Name("B".into()));
        store.insert(e(2), Name("C".into()));

        assert!(store.remove::<Name>(e(1)));
        assert!(!store.has::<Name>(e(1)));
        // Others still intact
        assert_eq!(store.get::<Name>(e(0)), Some(&Name("A".into())));
        assert_eq!(store.get::<Name>(e(2)), Some(&Name("C".into())));
        assert_eq!(store.count::<Name>(), 2);
    }

    #[test]
    fn remove_nonexistent() {
        let mut store = ComponentStore::new();
        assert!(!store.remove::<Name>(e(0)));
    }

    #[test]
    fn get_mut_component() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Health(100));
        if let Some(h) = store.get_mut::<Health>(e(0)) {
            h.0 -= 25;
        }
        assert_eq!(store.get::<Health>(e(0)), Some(&Health(75)));
    }

    #[test]
    fn iterate_components() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Health(100));
        store.insert(e(1), Health(200));
        store.insert(e(2), Health(300));

        let mut items: Vec<_> = store.iter::<Health>().collect();
        items.sort_by_key(|(e, _)| e.index());
        assert_eq!(
            items,
            vec![
                (e(0), &Health(100)),
                (e(1), &Health(200)),
                (e(2), &Health(300))
            ]
        );
    }

    #[test]
    fn iterate_empty() {
        let store = ComponentStore::new();
        assert_eq!(store.iter::<Name>().count(), 0);
    }

    #[test]
    fn multiple_component_types() {
        let mut store = ComponentStore::new();
        store.insert(e(0), Name("Alice".into()));
        store.insert(e(0), Health(100));
        store.insert(e(0), Marker);

        assert!(store.has::<Name>(e(0)));
        assert!(store.has::<Health>(e(0)));
        assert!(store.has::<Marker>(e(0)));

        store.remove::<Health>(e(0));
        assert!(store.has::<Name>(e(0)));
        assert!(!store.has::<Health>(e(0)));
    }

    #[test]
    fn swap_remove_preserves_other_entities() {
        let mut store = ComponentStore::new();
        // Insert 5 entities
        for i in 0..5 {
            store.insert(e(i), Health(i as i32));
        }
        // Remove from the middle
        store.remove::<Health>(e(1));
        store.remove::<Health>(e(3));

        assert_eq!(store.count::<Health>(), 3);
        assert_eq!(store.get::<Health>(e(0)), Some(&Health(0)));
        assert_eq!(store.get::<Health>(e(2)), Some(&Health(2)));
        assert_eq!(store.get::<Health>(e(4)), Some(&Health(4)));
    }
}
