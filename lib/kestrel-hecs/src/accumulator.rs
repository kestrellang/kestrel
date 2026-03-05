use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::query::QueryKey;

/// Type-erased trait for accumulator storage.
trait AnyAccumulator: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear_for_query(&mut self, query: &QueryKey);
}

/// Typed accumulator for side-effect values of type T.
///
/// Queries push values here during execution. When a query re-executes,
/// its previously accumulated values are cleared first. This is the
/// salsa accumulator pattern — diagnostics, warnings, etc. without
/// polluting query return types.
struct TypedAccumulator<T> {
    by_query: HashMap<QueryKey, Vec<T>>,
}

impl<T: Clone + 'static> TypedAccumulator<T> {
    fn new() -> Self {
        Self {
            by_query: HashMap::new(),
        }
    }

    fn push(&mut self, query: QueryKey, value: T) {
        self.by_query.entry(query).or_default().push(value);
    }

    fn clear_for_query(&mut self, query: &QueryKey) {
        self.by_query.remove(query);
    }

    fn all(&self) -> impl Iterator<Item = &T> {
        self.by_query.values().flat_map(|v| v.iter())
    }
}

impl<T: Clone + 'static> AnyAccumulator for TypedAccumulator<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn clear_for_query(&mut self, query: &QueryKey) {
        self.clear_for_query(query);
    }
}

/// Storage for all accumulators, keyed by the accumulated value's TypeId.
///
/// Supports multiple accumulator types simultaneously — e.g. one for
/// diagnostics, one for warnings, one for metrics.
pub struct AccumulatorStore {
    stores: HashMap<TypeId, Box<dyn AnyAccumulator>>,
}

impl AccumulatorStore {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    /// Push a value into the accumulator for type T, associated with a query.
    pub fn push<T: Clone + 'static>(&mut self, query: QueryKey, value: T) {
        self.store_mut::<T>().push(query, value);
    }

    /// Clear all accumulated values for a query (called before re-execution).
    pub fn clear_for_query(&mut self, query: &QueryKey) {
        for store in self.stores.values_mut() {
            store.clear_for_query(query);
        }
    }

    /// Iterate over all accumulated values of type T.
    pub fn all<T: Clone + 'static>(&self) -> impl Iterator<Item = &T> {
        self.store::<T>()
            .into_iter()
            .flat_map(|s| s.all())
    }

    fn store<T: Clone + 'static>(&self) -> Option<&TypedAccumulator<T>> {
        self.stores
            .get(&TypeId::of::<T>())
            .and_then(|s| s.as_any().downcast_ref::<TypedAccumulator<T>>())
    }

    fn store_mut<T: Clone + 'static>(&mut self) -> &mut TypedAccumulator<T> {
        self.stores
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedAccumulator::<T>::new()))
            .as_any_mut()
            .downcast_mut::<TypedAccumulator<T>>()
            .expect("type mismatch in accumulator store")
    }
}

impl Default for AccumulatorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qk(type_id: u64, key_hash: u64) -> QueryKey {
        QueryKey { type_id, key_hash }
    }

    #[test]
    fn push_and_iterate() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "error: foo".to_string());
        store.push(qk(1, 10), "error: bar".to_string());
        store.push(qk(2, 20), "error: baz".to_string());

        let all: Vec<_> = store.all::<String>().collect();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn clear_for_query() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "from query 1".to_string());
        store.push(qk(2, 20), "from query 2".to_string());

        store.clear_for_query(&qk(1, 10));

        let all: Vec<_> = store.all::<String>().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], "from query 2");
    }

    #[test]
    fn multiple_accumulator_types() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "a string".to_string());
        store.push(qk(1, 10), 42u32);

        assert_eq!(store.all::<String>().count(), 1);
        assert_eq!(store.all::<u32>().count(), 1);
    }

    #[test]
    fn empty_accumulator() {
        let store = AccumulatorStore::new();
        assert_eq!(store.all::<String>().count(), 0);
    }
}
