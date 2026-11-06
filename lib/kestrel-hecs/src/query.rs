use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::accumulator::AccumulatorStore;
use crate::component::{Component, ComponentStore};
use crate::entity::Entity;
use crate::fingerprint::Fingerprint;
use crate::guard::OnDrop;
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
pub trait QueryFn: Hash + Eq + Clone + Send + Sync + 'static {
    type Output: Clone + Hash + Send + Sync + 'static;

    /// Execute the query. All component reads and sub-query calls go
    /// through `ctx`, which records dependencies automatically.
    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output;

    /// Debug description for cycle detection diagnostics.
    fn describe(&self) -> String {
        std::any::type_name::<Self>().to_string()
    }

    /// What re-entering this query's key while it is still executing means.
    /// The default panics ("Query cycle detected"); recursive-type queries
    /// opt into fixpoint iteration or a fallback answer. See [`CycleMode`].
    fn cycle_mode(&self) -> CycleMode<Self::Output> {
        CycleMode::Panic
    }

    /// Called on a `Fallback` cycle head after its cycle was detected, inside
    /// the head's own execution scope — so anything it accumulates (e.g. a
    /// diagnostic) is attributed to the head and replaced on re-execution.
    fn on_cycle_fallback(&self, _ctx: &QueryContext<'_>) {}
}

/// Cycle recovery policy for a query (Salsa-style). A cycle is a query that,
/// directly or through other queries, asks for its own key while it is still
/// executing. The first query of the cycle on the stack is its **head**.
pub enum CycleMode<V> {
    /// A cycle is a compiler bug: panic with the active stack.
    Panic,
    /// Iterate to a fixpoint. Re-entry answers the head's provisional value,
    /// starting at `initial`; the head re-executes with its last output as
    /// the new provisional value until the output stops changing. Outputs
    /// must walk a finite lattice monotonically from `initial`, or the
    /// iteration cap ICEs.
    Fixpoint { initial: V },
    /// The cycle is an error: re-entry answers `value`, the head answers
    /// `value` (after `on_cycle_fallback`), and nothing iterates.
    Fallback { value: V },
}

/// Fixpoint iterations a cycle head may take before it is declared divergent.
const MAX_CYCLE_ITERATIONS: usize = 100;

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
type VerifierFn = Arc<dyn Fn(&QueryContext<'_>) -> Revision + Send + Sync>;

/// One query type's memo map, type-erased. The fn pointers are the only
/// operations that need the concrete `Q`: cloning (snapshots) and finding
/// the memos that read given entities (`World::despawn`).
struct ErasedStore {
    data: Box<dyn Any + Send + Sync>,
    clone_fn: fn(&(dyn Any + Send + Sync)) -> Box<dyn Any + Send + Sync>,
    readers_fn: fn(&(dyn Any + Send + Sync), &FxHashSet<Entity>, &mut Vec<QueryKey>),
}

impl ErasedStore {
    fn new<Q: QueryFn>() -> Self {
        Self {
            data: Box::new(MemoMap::<Q>::default()),
            clone_fn: |any| Box::new(downcast_memos::<Q>(any).clone()),
            readers_fn: |any, entities, out| {
                out.extend(
                    downcast_memos::<Q>(any)
                        .iter()
                        .filter(|(_, memo)| memo.reads_any(entities))
                        .map(|(q, _)| query_key(q)),
                );
            },
        }
    }

    fn clone_store(&self) -> Self {
        Self {
            data: (self.clone_fn)(&*self.data),
            ..*self
        }
    }
}

type MemoMap<Q> = FxHashMap<Q, MemoEntry<<Q as QueryFn>::Output>>;

fn downcast_memos<Q: QueryFn>(any: &(dyn Any + Send + Sync)) -> &MemoMap<Q> {
    any.downcast_ref::<MemoMap<Q>>()
        .expect("type mismatch in query storage")
}

impl<V> MemoEntry<V> {
    /// Whether this memo read a component (or the children) of one of
    /// `entities` itself — not through a sub-query.
    fn reads_any(&self, entities: &FxHashSet<Entity>) -> bool {
        self.deps.iter().any(
            |dep| matches!(dep, Dependency::Component { entity, .. } if entities.contains(entity)),
        )
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
            .and_then(|s| s.data.downcast_ref::<MemoMap<Q>>())
            .and_then(|map| map.get(key))
    }

    fn insert_memo<Q: QueryFn>(&mut self, key: Q, entry: MemoEntry<Q::Output>) {
        self.stores
            .entry(TypeId::of::<Q>())
            .or_insert_with(ErasedStore::new::<Q>)
            .data
            .downcast_mut::<MemoMap<Q>>()
            .expect("type mismatch in query storage")
            .insert(key, entry);
    }

    fn update_verified<Q: QueryFn>(&mut self, key: &Q, revision: Revision) {
        if let Some(store) = self.stores.get_mut(&TypeId::of::<Q>())
            && let Some(map) = store.data.downcast_mut::<MemoMap<Q>>()
            && let Some(memo) = map.get_mut(key)
        {
            memo.verified_at = revision;
        }
    }

    fn register_verifier(&mut self, key: QueryKey, verifier: VerifierFn) {
        self.verifiers.insert(key, verifier);
    }

    /// Every memoized query that directly read one of `entities`.
    pub(crate) fn readers_of(&self, entities: &FxHashSet<Entity>) -> Vec<QueryKey> {
        let mut out = Vec::new();
        for store in self.stores.values() {
            (store.readers_fn)(&*store.data, entities, &mut out);
        }
        out
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
struct ActiveQuery {
    key: QueryKey,
    /// Debug label for cycle diagnostics (from QueryFn::describe).
    label: String,
    /// Set once another frame re-entered this key: this frame is a cycle
    /// head, and this is the value re-entry hands out (`Q::Output`, erased).
    provisional: Option<Box<dyn Any + Send + Sync>>,
    /// Whether the head's mode is `Fallback` (no iteration).
    fallback: bool,
    /// Lowest stack index of a cycle head whose provisional value this
    /// frame's result was computed from (directly or via a callee). Such a
    /// result is provisional too: it is memoized only once that head
    /// converges.
    depends_on_head: Option<usize>,
}

/// A result computed under a still-iterating cycle head. Held outside the
/// memo table until `head` converges, then committed by `commit`.
struct Provisional {
    /// Stack index of the head this result waits on.
    head: usize,
    /// The `Q::Output`, erased — answered to readers in the same iteration.
    value: Box<dyn Any + Send + Sync>,
    deps: Vec<Dependency>,
    /// Memoizes the result with the cycle's shared external deps.
    commit: Box<dyn FnOnce(&QueryContext<'_>, Vec<Dependency>)>,
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
    /// Per-entity `last_changed` stamps — the invalidation signal.
    pub(crate) entities: &'a [crate::world::EntityRecord],
    pub(crate) hierarchy: &'a crate::world::Hierarchy,
    pub(crate) queries: &'a RefCell<QueryStorage>,
    pub(crate) accumulators: &'a RefCell<AccumulatorStore>,
    /// Counter for actual query executions (not cache hits).
    pub(crate) exec_count: &'a Cell<u64>,
    /// Dependencies recorded during the current query's execution.
    deps: RefCell<Vec<Dependency>>,
    /// Call stack for cycle detection.
    active: RefCell<Vec<ActiveQuery>>,
    /// Stack index of each active key — O(1) re-entry detection.
    active_index: RefCell<FxHashMap<QueryKey, usize>>,
    /// Results waiting on an iterating cycle head (see [`Provisional`]).
    provisional: RefCell<FxHashMap<QueryKey, Provisional>>,
}

impl<'a> QueryContext<'a> {
    pub(crate) fn new(
        revision: Revision,
        components: &'a ComponentStore,
        entities: &'a [crate::world::EntityRecord],
        hierarchy: &'a crate::world::Hierarchy,
        queries: &'a RefCell<QueryStorage>,
        accumulators: &'a RefCell<AccumulatorStore>,
        exec_count: &'a Cell<u64>,
    ) -> Self {
        Self {
            revision,
            components,
            entities,
            hierarchy,
            queries,
            accumulators,
            exec_count,
            deps: RefCell::new(Vec::new()),
            active: RefCell::new(Vec::new()),
            active_index: RefCell::new(FxHashMap::default()),
            provisional: RefCell::new(FxHashMap::default()),
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
    pub fn accumulate<T: Clone + Send + Sync + 'static>(&self, value: T) {
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
        // Phase 0: a cycle re-entry or a provisional result answers before
        // any memo — the memo (if any) predates the iteration in progress.
        if let Some(answer) = self.cycle_answer(q, qk) {
            return answer;
        }

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

            // The mark is a *promise*, made good either by `deps_unchanged`
            // agreeing or by `execute_query` rewriting the memo. If anything
            // between here and one of those panics and a host catches it, the
            // promise is never kept: the memo keeps its OLD value behind a
            // `verified_at` that says "confirmed this revision", so every
            // later reader in this revision silently takes a stale value.
            // Roll the mark back on the unwind path only — the normal path
            // legitimately keeps it (F22). The guard must outlive the
            // re-execution below, not just the verification.
            let queries = self.queries;
            let _verify_guard = OnDrop::new(|| {
                if std::thread::panicking() {
                    queries.borrow_mut().update_verified::<Q>(q, verified_at);
                }
            });

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

            // Deps changed — re-execute, still under the guard.
            return self.execute_query(q, qk);
        }

        // Phase 2: Execute the query
        self.execute_query(q, qk)
    }

    /// Answer `q` from the cycle machinery, if it is involved:
    /// - `q` is active (re-entry) and opted into recovery → the head's
    ///   provisional value (seeded from its `CycleMode`);
    /// - `q` has a provisional result waiting on an active head → that.
    ///
    /// Either way every frame above the head now depends on it, and the
    /// answer reports "changed now" so no verifier trusts it. A re-entered
    /// `Panic` key returns `None`: the memo path and `execute_query` keep
    /// today's behavior.
    fn cycle_answer<Q: QueryFn>(&self, q: &Q, qk: &QueryKey) -> Option<(Q::Output, Revision)> {
        let active_idx = self.active_index.borrow().get(qk).copied();
        if let Some(idx) = active_idx {
            let (seed, fallback) = match q.cycle_mode() {
                CycleMode::Panic => return None,
                CycleMode::Fixpoint { initial } => (initial, false),
                CycleMode::Fallback { value } => (value, true),
            };
            let mut stack = self.active.borrow_mut();
            let frame = &mut stack[idx];
            let answer = match &frame.provisional {
                Some(value) => downcast_output::<Q>(value).clone(),
                None => {
                    frame.provisional = Some(Box::new(seed.clone()));
                    frame.fallback = fallback;
                    seed
                },
            };
            mark_dependents(&mut stack, idx);
            return Some((answer, self.revision));
        }

        let provisional = self.provisional.borrow();
        if provisional.is_empty() {
            return None;
        }
        let entry = provisional.get(qk)?;
        let answer = downcast_output::<Q>(&entry.value).clone();
        let head = entry.head;
        drop(provisional);
        mark_dependents(&mut self.active.borrow_mut(), head);
        Some((answer, self.revision))
    }

    /// Execute a query from scratch (no valid cache).
    fn execute_query<Q: QueryFn>(&self, q: &Q, qk: &QueryKey) -> (Q::Output, Revision) {
        // Cycle detection. Recoverable keys never get here on re-entry
        // (`cycle_answer` answers them), so this is a `Panic`-mode cycle.
        if self.active_index.borrow().contains_key(qk) {
            let stack = self.active.borrow();
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
        let idx = {
            let mut stack = self.active.borrow_mut();
            stack.push(ActiveQuery {
                key: qk.clone(),
                label: q.describe(),
                provisional: None,
                fallback: false,
                depends_on_head: None,
            });
            stack.len() - 1
        };
        self.active_index.borrow_mut().insert(qk.clone(), idx);
        // The pop MUST also happen when `q.execute` panics. Hosts catch
        // per-unit-of-work panics and keep using the same context (see
        // `kestrel_hecs::guard`); a leaked entry makes every later query
        // that touches this key report a fabricated cycle, and misfiles
        // top-level `accumulate` calls under the dead key. Pushes/pops are
        // strictly LIFO (a nested execute completes inside this one), and
        // guards unwind innermost-first, so `pop` removes our own entry.
        // Provisional results waiting on this frame are normally committed
        // or re-homed before it pops; on unwind they are orphans — drop them.
        let active = &self.active;
        let active_index = &self.active_index;
        let provisional = &self.provisional;
        let _active_guard = OnDrop::new(|| {
            if let Some(frame) = active.borrow_mut().pop() {
                active_index.borrow_mut().remove(&frame.key);
            }
            provisional.borrow_mut().retain(|_, p| p.head < idx);
        });

        // Take (don't drop) this query's old accumulated values: the
        // execution about to replace them can panic partway through, and
        // losing previously-valid diagnostics to a caught ICE is worse than
        // keeping stale ones for a revision.
        let accumulators = self.accumulators;
        let saved_acc = Cell::new(Some(accumulators.borrow_mut().take_for_query(qk)));
        let acc_key = qk.clone();
        let _acc_guard = OnDrop::new(|| {
            if std::thread::panicking()
                && let Some(snapshot) = saved_acc.take()
            {
                accumulators
                    .borrow_mut()
                    .restore_for_query(&acc_key, snapshot);
            }
        });

        // Execute in a fresh dependency scope. The caller's list is restored
        // by the guard on both paths — `execute_query` records no deps of
        // its own after `q.execute` returns.
        let deps = &self.deps;
        let saved_deps = Cell::new(Some(deps.take()));
        let _deps_guard = OnDrop::new(|| {
            if let Some(saved) = saved_deps.take() {
                deps.replace(saved);
            }
        });
        let result = self.execute_to_fixpoint(q, qk, idx);
        let new_deps = self.deps.take();

        let (is_head, depends_on_head) = {
            let stack = self.active.borrow();
            let frame = &stack[idx];
            (frame.provisional.is_some(), frame.depends_on_head)
        };

        // Computed from an outer head's provisional value: park it (and any
        // results parked on this frame) until that head converges.
        if let Some(head) = depends_on_head {
            // A stale memo of `q` may carry `ensure_fresh`'s tentative
            // "verified this revision" mark; left in place, a later
            // iteration would read the old value through it. Revoke it so
            // the memo re-verifies until the commit replaces it.
            self.queries
                .borrow_mut()
                .update_verified::<Q>(q, Revision::initial());
            let mut provisional = self.provisional.borrow_mut();
            for p in provisional.values_mut().filter(|p| p.head == idx) {
                p.head = head;
            }
            let (q_commit, qk_commit, value) = (q.clone(), qk.clone(), result.clone());
            provisional.insert(
                qk.clone(),
                Provisional {
                    head,
                    value: Box::new(result.clone()),
                    deps: new_deps,
                    commit: Box::new(move |ctx, deps| {
                        ctx.commit_memo(&q_commit, &qk_commit, value, deps);
                    }),
                },
            );
            return (result, self.revision);
        }

        let deps = if is_head {
            self.commit_cycle(idx, qk, new_deps)
        } else {
            new_deps
        };
        let changed_at = self.commit_memo(q, qk, result.clone(), deps);
        (result, changed_at)
    }

    /// Run `q.execute`, re-running it while it is a `Fixpoint` cycle head
    /// whose output differs from the provisional value it was computed
    /// under. A `Fallback` head answers its fallback value.
    fn execute_to_fixpoint<Q: QueryFn>(&self, q: &Q, qk: &QueryKey, idx: usize) -> Q::Output {
        let mut iterations = 0;
        loop {
            self.exec_count.set(self.exec_count.get() + 1);
            let result = q.execute(self);

            let mut stack = self.active.borrow_mut();
            let frame = &mut stack[idx];
            let Some(prev) = &frame.provisional else {
                return result;
            };
            if frame.fallback {
                let value = downcast_output::<Q>(prev).clone();
                drop(stack);
                q.on_cycle_fallback(self);
                return value;
            }
            if Fingerprint::of(&result) == Fingerprint::of(downcast_output::<Q>(prev)) {
                return result;
            }
            iterations += 1;
            assert!(
                iterations < MAX_CYCLE_ITERATIONS,
                "query cycle did not converge after {MAX_CYCLE_ITERATIONS} iterations: {} \
                 (its output must walk a finite lattice monotonically)",
                frame.label
            );
            frame.provisional = Some(Box::new(result));
            drop(stack);
            // Everything computed from the stale provisional value is redone,
            // including this head's own deps and accumulated values.
            let stale: Vec<QueryKey> = {
                let mut provisional = self.provisional.borrow_mut();
                let stale = provisional
                    .iter()
                    .filter(|(_, p)| p.head == idx)
                    .map(|(k, _)| k.clone())
                    .collect();
                provisional.retain(|_, p| p.head != idx);
                stale
            };
            self.deps.borrow_mut().clear();
            let mut accumulators = self.accumulators.borrow_mut();
            // Discarded on purpose: computed from the stale value. An unwind
            // in the next iteration restores the head's pre-execution
            // snapshot (taken by `execute_query`), so nothing valid is lost.
            for key in stale.iter().chain(std::iter::once(qk)) {
                let _ = accumulators.take_for_query(key);
            }
        }
    }

    /// Commit every result parked on the converged head at `idx`. All
    /// members of a cycle share one dependency list — the union of their
    /// deps minus edges between members — so verifying any member never
    /// re-enters the cycle, and any external change reruns all of them.
    /// Returns that list for the head's own memo.
    fn commit_cycle(
        &self,
        idx: usize,
        head_key: &QueryKey,
        head_deps: Vec<Dependency>,
    ) -> Vec<Dependency> {
        let members: Vec<(QueryKey, Provisional)> = {
            let mut provisional = self.provisional.borrow_mut();
            let keys: Vec<QueryKey> = provisional
                .iter()
                .filter(|(_, p)| p.head == idx)
                .map(|(k, _)| k.clone())
                .collect();
            keys.into_iter()
                .filter_map(|k| provisional.remove(&k).map(|p| (k, p)))
                .collect()
        };
        let internal: FxHashSet<&QueryKey> = members
            .iter()
            .map(|(k, _)| k)
            .chain(std::iter::once(head_key))
            .collect();
        let mut seen = FxHashSet::default();
        let external: Vec<Dependency> = head_deps
            .iter()
            .chain(members.iter().flat_map(|(_, p)| p.deps.iter()))
            .filter(|dep| !matches!(dep, Dependency::Query(k) if internal.contains(k)))
            .filter(|dep| seen.insert(*dep))
            .cloned()
            .collect();
        for (_, member) in members {
            (member.commit)(self, external.clone());
        }
        external
    }

    /// Memoize a final result (with backdating) and register its verifier.
    fn commit_memo<Q: QueryFn>(
        &self,
        q: &Q,
        qk: &QueryKey,
        result: Q::Output,
        deps: Vec<Dependency>,
    ) -> Revision {
        // Backdating: if the result fingerprint matches the old memo,
        // keep the old changed_at so dependents skip re-execution.
        let new_fp = Fingerprint::of(&result);
        let changed_at = {
            let storage = self.queries.borrow();
            match storage.get_memo::<Q>(q) {
                Some(old_memo) if old_memo.fingerprint == new_fp => old_memo.changed_at,
                _ => self.revision,
            }
        };

        let entry = MemoEntry {
            value: result,
            fingerprint: new_fp,
            deps,
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
        changed_at
    }

    /// Check if all dependencies are still unchanged.
    ///
    /// For component deps, the entity is changed if it was stamped after
    /// `since` (the memo's `verified_at`). The stamp is durable: unlike the
    /// per-revision `ChangeSet`, it is still visible revisions later, so a
    /// change made in a revision where no query read the entity is not lost.
    /// A despawned entity keeps its record, stamped at despawn.
    /// For sub-query deps, recursively verifies the sub-query via its
    /// registered type-erased verifier and checks if its changed_at
    /// is still within bounds.
    fn deps_unchanged(&self, deps: &[Dependency], since: Revision) -> bool {
        for dep in deps {
            match dep {
                Dependency::Component { entity, .. } => {
                    let stamped_after = self
                        .entities
                        .get(entity.index())
                        .is_none_or(|record| record.last_changed > since);
                    if stamped_after {
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

/// Every frame above `head` now depends on its provisional value.
fn mark_dependents(stack: &mut [ActiveQuery], head: usize) {
    for frame in &mut stack[head + 1..] {
        frame.depends_on_head = Some(frame.depends_on_head.map_or(head, |h| h.min(head)));
    }
}

/// Takes the `Box` (not `&dyn Any`) on purpose: `&Box<dyn Any>` would
/// unsize-coerce to a `&dyn Any` of the box itself and never downcast.
#[allow(clippy::borrowed_box)]
fn downcast_output<Q: QueryFn>(value: &Box<dyn Any + Send + Sync>) -> &Q::Output {
    (**value)
        .downcast_ref::<Q::Output>()
        .expect("type mismatch in provisional query value")
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

    // -- Cycle recovery --

    #[derive(Clone, Debug, PartialEq)]
    struct Edges(Vec<Entity>);

    /// Max of `Value` over everything reachable — a monotone fixpoint over
    /// a graph that may contain cycles.
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct Reach {
        entity: Entity,
    }

    impl QueryFn for Reach {
        type Output = i32;

        fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
            let own = ctx.get::<Value>(self.entity).map_or(0, |v| v.0);
            let edges = ctx
                .get::<Edges>(self.entity)
                .map(|e| e.0.clone())
                .unwrap_or_default();
            edges
                .into_iter()
                .map(|entity| ctx.query(Reach { entity }))
                .fold(own, i32::max)
        }

        fn cycle_mode(&self) -> CycleMode<i32> {
            CycleMode::Fixpoint { initial: 0 }
        }
    }

    /// Depth of the longest path; a cycle is an error answering -1.
    #[derive(Clone, PartialEq, Eq, Hash)]
    struct Depth {
        entity: Entity,
    }

    impl QueryFn for Depth {
        type Output = i32;

        fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
            let edges = ctx
                .get::<Edges>(self.entity)
                .map(|e| e.0.clone())
                .unwrap_or_default();
            let mut depth = 0;
            for entity in edges {
                let d = ctx.query(Depth { entity });
                if d < 0 {
                    return -1;
                }
                depth = depth.max(d + 1);
            }
            depth
        }

        fn cycle_mode(&self) -> CycleMode<i32> {
            CycleMode::Fallback { value: -1 }
        }

        fn on_cycle_fallback(&self, ctx: &QueryContext<'_>) {
            ctx.accumulate(format!("cycle through {:?}", self.entity));
        }
    }

    fn graph(world: &mut World, values: &[i32]) -> Vec<Entity> {
        values
            .iter()
            .map(|&v| {
                let e = world.spawn();
                world.set(e, Value(v));
                e
            })
            .collect()
    }

    #[test]
    fn fixpoint_direct_cycle() {
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[3]);
        world.set(n[0], Edges(vec![n[0]]));

        let ctx = world.query_context();
        assert_eq!(ctx.query(Reach { entity: n[0] }), 3);
    }

    #[test]
    fn fixpoint_mutual_cycle_answers_every_member() {
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[1, 5]);
        world.set(n[0], Edges(vec![n[1]]));
        world.set(n[1], Edges(vec![n[0]]));

        let ctx = world.query_context();
        assert_eq!(ctx.query(Reach { entity: n[0] }), 5);
        // `n[1]` was computed as a cycle member and committed with the head.
        let execs = world.query_exec_count();
        assert_eq!(ctx.query(Reach { entity: n[1] }), 5);
        assert_eq!(world.query_exec_count(), execs, "member must be memoized");
    }

    #[test]
    fn fixpoint_nested_heads() {
        // 0 -> 1 -> 2 -> 1, 2 -> 0: the inner head (1) depends on the
        // outer one (0), so its result waits for 0 to converge.
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[1, 2, 9]);
        world.set(n[0], Edges(vec![n[1]]));
        world.set(n[1], Edges(vec![n[2]]));
        world.set(n[2], Edges(vec![n[1], n[0]]));

        let ctx = world.query_context();
        assert_eq!(ctx.query(Reach { entity: n[0] }), 9);
        assert_eq!(ctx.query(Reach { entity: n[1] }), 9);
        assert_eq!(ctx.query(Reach { entity: n[2] }), 9);
    }

    #[test]
    fn fixpoint_member_change_reruns_cycle() {
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[1, 5, 0]);
        world.set(n[0], Edges(vec![n[1]]));
        world.set(n[1], Edges(vec![n[0]]));
        // n[2] sits outside the cycle and reads into it.
        world.set(n[2], Edges(vec![n[0]]));
        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(Reach { entity: n[2] }), 5);
        }

        // Lowering a member's value must not leave a stale memo behind
        // (a guard-based "assume" would keep answering 5).
        world.begin_revision();
        world.set(n[1], Value(2));
        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(Reach { entity: n[2] }), 2);
            assert_eq!(ctx.query(Reach { entity: n[1] }), 2);
            assert_eq!(ctx.query(Reach { entity: n[0] }), 2);
        }

        // Ask the other member first this time.
        world.begin_revision();
        world.set(n[0], Value(8));
        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(Reach { entity: n[1] }), 8);
            assert_eq!(ctx.query(Reach { entity: n[2] }), 8);
        }
    }

    #[test]
    fn fixpoint_unrelated_change_reuses_cycle() {
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[1, 5, 0]);
        world.set(n[0], Edges(vec![n[1]]));
        world.set(n[1], Edges(vec![n[0]]));
        {
            let ctx = world.query_context();
            ctx.query(Reach { entity: n[0] });
        }

        world.begin_revision();
        world.set(n[2], Value(100));
        let before = world.query_exec_count();
        {
            let ctx = world.query_context();
            assert_eq!(ctx.query(Reach { entity: n[0] }), 5);
            assert_eq!(ctx.query(Reach { entity: n[1] }), 5);
        }
        assert_eq!(
            world.query_exec_count(),
            before,
            "cycle must verify from cache"
        );
    }

    #[test]
    fn fallback_answers_head_and_reports_once() {
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[0, 0, 0]);
        world.set(n[0], Edges(vec![n[1]]));
        world.set(n[1], Edges(vec![n[0]]));
        world.set(n[2], Edges(vec![]));

        let ctx = world.query_context();
        assert_eq!(ctx.query(Depth { entity: n[0] }), -1);
        assert_eq!(ctx.query(Depth { entity: n[1] }), -1);
        assert_eq!(ctx.query(Depth { entity: n[2] }), 0);
        let errors = world.accumulated::<String>();
        assert_eq!(errors, vec![format!("cycle through {:?}", n[0])]);
    }

    #[test]
    #[should_panic(expected = "Query cycle detected")]
    fn panic_mode_still_panics_inside_recoverable_cycle() {
        // A `Panic` query re-entered directly still panics, even when a
        // recoverable query is also on the stack.
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct Outer {
            entity: Entity,
        }
        impl QueryFn for Outer {
            type Output = i32;
            fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
                ctx.query(CyclicQuery {
                    entity: self.entity,
                })
            }
            fn cycle_mode(&self) -> CycleMode<i32> {
                CycleMode::Fixpoint { initial: 0 }
            }
        }
        let mut world = World::new();
        world.begin_revision();
        let e = world.spawn();
        let ctx = world.query_context();
        ctx.query(Outer { entity: e });
    }

    #[test]
    fn cycle_state_survives_a_caught_panic() {
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct Boom {
            entity: Entity,
        }
        impl QueryFn for Boom {
            type Output = i32;
            fn execute(&self, ctx: &QueryContext<'_>) -> i32 {
                // Enter a cycle, then panic while its member is parked.
                let r = ctx.query(Reach {
                    entity: self.entity,
                });
                if ctx.get::<Name>(self.entity).is_some() {
                    panic!("boom");
                }
                r
            }
        }
        let mut world = World::new();
        world.begin_revision();
        let n = graph(&mut world, &[4]);
        world.set(n[0], Edges(vec![n[0]]));
        world.set(n[0], Name("x".into()));
        let ctx = world.query_context();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ctx.query(Boom { entity: n[0] })
        }));
        assert!(caught.is_err());
        assert!(ctx.active.borrow().is_empty());
        assert!(ctx.active_index.borrow().is_empty());
        assert!(ctx.provisional.borrow().is_empty());
        assert_eq!(ctx.query(Reach { entity: n[0] }), 4);
    }
}
