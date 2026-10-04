use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::query::QueryKey;

/// Type-erased trait for accumulator storage.
trait AnyAccumulator: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Remove this query's values and hand them back as an erased `Vec<T>`.
    /// `None` when nothing was filed — the common case on a hot path, and
    /// worth not boxing an empty vector for.
    fn take_for_query(&mut self, query: &QueryKey) -> Option<Box<dyn Any + Send + Sync>>;
    /// Put an erased `Vec<T>` produced by `take_for_query` back, replacing
    /// whatever is currently filed under `query`.
    fn restore_for_query(&mut self, query: &QueryKey, values: Box<dyn Any + Send + Sync>);
}

/// Everything filed under one `QueryKey`, across every accumulator type.
///
/// Produced by `AccumulatorStore::take_for_query` before a query executes and
/// handed back by `restore_for_query` if that execution unwinds. Opaque on
/// purpose: the per-type payloads are `Vec<T>` erased to `Box<dyn Any + Send + Sync>`.
pub struct AccumulatorSnapshot {
    per_type: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

/// Typed accumulator for side-effect values of type T.
///
/// Queries push values here during execution. When a query re-executes, its
/// previously accumulated values are taken first (and put back if that
/// execution unwinds — see `AccumulatorStore::take_for_query`). This is the
/// salsa accumulator pattern — diagnostics, warnings, etc. without
/// polluting query return types.
struct TypedAccumulator<T> {
    by_query: HashMap<QueryKey, Vec<T>>,
}

impl<T: Clone + Send + Sync + 'static> TypedAccumulator<T> {
    fn new() -> Self {
        Self {
            by_query: HashMap::new(),
        }
    }

    fn push(&mut self, query: QueryKey, value: T) {
        self.by_query.entry(query).or_default().push(value);
    }

    fn take_for_query(&mut self, query: &QueryKey) -> Option<Vec<T>> {
        self.by_query.remove(query)
    }

    fn restore_for_query(&mut self, query: &QueryKey, values: Vec<T>) {
        self.by_query.insert(query.clone(), values);
    }

    fn all(&self) -> impl Iterator<Item = &T> {
        self.by_query.values().flat_map(|v| v.iter())
    }
}

impl<T: Clone + Send + Sync + 'static> AnyAccumulator for TypedAccumulator<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn take_for_query(&mut self, query: &QueryKey) -> Option<Box<dyn Any + Send + Sync>> {
        self.take_for_query(query)
            .map(|values| Box::new(values) as Box<dyn Any + Send + Sync>)
    }
    fn restore_for_query(&mut self, query: &QueryKey, values: Box<dyn Any + Send + Sync>) {
        let values = *values
            .downcast::<Vec<T>>()
            .expect("type mismatch restoring an accumulator snapshot");
        self.restore_for_query(query, values);
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
    pub fn push<T: Clone + Send + Sync + 'static>(&mut self, query: QueryKey, value: T) {
        self.store_mut::<T>().push(query, value);
    }

    /// Remove everything filed under `query` and return it (called before
    /// re-execution, which is about to re-produce it).
    ///
    /// This is the only clearing path: a query's old values must be taken
    /// rather than dropped, because the execution that replaces them can
    /// panic partway through. `execute_query` hands the snapshot back via
    /// `restore_for_query` when that happens, so a caught ICE costs the
    /// query's *new* diagnostics, not its previously-valid ones.
    #[must_use = "dropping the snapshot is the pre-F22 bug: an unwind loses the old values"]
    pub fn take_for_query(&mut self, query: &QueryKey) -> AccumulatorSnapshot {
        AccumulatorSnapshot {
            // Types with nothing filed are simply absent: `restore_for_query`
            // treats absence as "was empty, clear it", which is the same
            // answer without the allocation. Queries that accumulate nothing
            // are the overwhelming majority, and this keeps their snapshot
            // an empty (non-allocating) map.
            per_type: self
                .stores
                .iter_mut()
                .filter_map(|(&type_id, store)| {
                    store.take_for_query(query).map(|v| (type_id, v))
                })
                .collect(),
        }
    }

    /// Put a `take_for_query` snapshot back, restoring the exact pre-take
    /// state of `query` and discarding anything filed under it since.
    pub fn restore_for_query(&mut self, query: &QueryKey, snapshot: AccumulatorSnapshot) {
        let mut per_type = snapshot.per_type;
        for (type_id, store) in self.stores.iter_mut() {
            match per_type.remove(type_id) {
                Some(values) => store.restore_for_query(query, values),
                // Nothing was filed under `query` for this type when the
                // snapshot was taken (or the type did not exist yet), so
                // anything there now came from the run being rolled back.
                None => drop(store.take_for_query(query)),
            }
        }
    }

    /// Iterate over all accumulated values of type T.
    pub fn all<T: Clone + Send + Sync + 'static>(&self) -> impl Iterator<Item = &T> {
        self.store::<T>().into_iter().flat_map(|s| s.all())
    }

    fn store<T: Clone + Send + Sync + 'static>(&self) -> Option<&TypedAccumulator<T>> {
        self.stores
            .get(&TypeId::of::<T>())
            .and_then(|s| s.as_any().downcast_ref::<TypedAccumulator<T>>())
    }

    fn store_mut<T: Clone + Send + Sync + 'static>(&mut self) -> &mut TypedAccumulator<T> {
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
    fn take_for_query_clears_only_that_query() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "from query 1".to_string());
        store.push(qk(2, 20), "from query 2".to_string());

        let _snapshot = store.take_for_query(&qk(1, 10));

        let all: Vec<_> = store.all::<String>().collect();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0], "from query 2");
    }

    #[test]
    fn take_then_restore_round_trips() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "a".to_string());
        store.push(qk(1, 10), "b".to_string());
        store.push(qk(1, 10), 7u32);
        store.push(qk(2, 20), "other".to_string());

        let snapshot = store.take_for_query(&qk(1, 10));
        assert_eq!(store.all::<String>().count(), 1);
        assert_eq!(store.all::<u32>().count(), 0);

        store.restore_for_query(&qk(1, 10), snapshot);

        // Every accumulator type comes back, in order, and nothing else moved.
        let strings: Vec<_> = {
            let mut v: Vec<_> = store.all::<String>().cloned().collect();
            v.sort();
            v
        };
        assert_eq!(strings, vec!["a", "b", "other"]);
        assert_eq!(store.all::<u32>().copied().collect::<Vec<_>>(), vec![7]);
    }

    #[test]
    fn restore_replaces_values_pushed_after_the_take() {
        // Models the unwind path: execute pushed some values before it
        // panicked; restoring must reinstate the pre-execute state exactly,
        // not merge the half-finished run into it.
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "old".to_string());

        let snapshot = store.take_for_query(&qk(1, 10));
        store.push(qk(1, 10), "partial".to_string());
        store.restore_for_query(&qk(1, 10), snapshot);

        let all: Vec<_> = store.all::<String>().collect();
        assert_eq!(all, vec!["old"]);
    }

    #[test]
    fn restore_discards_a_type_that_appeared_after_the_take() {
        let mut store = AccumulatorStore::new();
        store.push(qk(1, 10), "old".to_string());

        let snapshot = store.take_for_query(&qk(1, 10));
        store.push(qk(1, 10), 99u32); // brand-new accumulator type
        store.restore_for_query(&qk(1, 10), snapshot);

        assert_eq!(store.all::<String>().collect::<Vec<_>>(), vec!["old"]);
        assert_eq!(store.all::<u32>().count(), 0);
    }

    #[test]
    fn restoring_an_empty_snapshot_clears_the_key() {
        // A query with no prior values snapshots empty; restoring that must
        // leave the key empty rather than keeping a partial run's pushes.
        let mut store = AccumulatorStore::new();
        store.push(qk(2, 20), "unrelated".to_string());

        let snapshot = store.take_for_query(&qk(1, 10));
        store.push(qk(1, 10), "partial".to_string());
        store.restore_for_query(&qk(1, 10), snapshot);

        let all: Vec<_> = store.all::<String>().collect();
        assert_eq!(all, vec!["unrelated"]);
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
