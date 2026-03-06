# kestrel-hecs Issues

## 1. `hash_of` uses stack pointer — backdating never fires
`query.rs` — `hash_of` computes fingerprints using the stack pointer, so every re-execution produces a different fingerprint regardless of value equality. The fingerprint comparison in `execute_query` (`old_memo.fingerprint == new_fp`) is always false, meaning early cutoff never triggers. The entire incremental architecture is O(batch) at runtime until this is fixed.

**Fix:** Require `QueryFn::Output: Hash` and compute fingerprints from the actual value. Alternatively, add a `Fingerprint` associated type or a separate `FingerprintOf` trait for types where `Hash` isn't semantically appropriate.

## 2. No cycle recovery — panics on query cycles
If two queries depend on each other (e.g., mutually recursive types, protocol conformance cycles), the `active` stack detects the cycle and panics. Production compilers need to handle this gracefully.

**Fix:** Add `QueryFn::cycle_fallback() -> Option<Self::Output>` that defaults to `None` (panic) but can be overridden per query to return an error sentinel.

## 3. `build_declarations` is mutation-phase, not a query
Lex and parse are query-driven, but AST building mutates the world directly. Any file change re-runs all declaration building for that file even if declarations didn't change. Entity identity is lost across rebuilds, which prevents downstream query caches from surviving.

**Fix:** Make declaration building a query (or query-like) with update-in-place semantics. Assign entity identity based on (parent module + name + kind) so the same struct keeps the same `Entity` across revisions. Similar to Salsa's `#[salsa::tracked]` identity model.

## 4. `ExtensionsFor` does a full tree scan
Every call to `ExtensionsFor` walks the entire module hierarchy to find extensions matching a target. O(declarations) per call.

**Fix:** Maintain an inverted index `HashMap<Entity, Vec<Entity>>` (target → extensions) during mutation phase. Query it in O(1).

## 5. No interning for common values
`String` names, `AstType`, `Span`, and eventually `HirTy` will be heavily duplicated across entities. Each duplicate wastes memory and makes fingerprint comparison slower (comparing content instead of an index).

**Fix:** Add an `Interner<T: Hash + Eq>` to the world, returning `Interned<T>` (a `u32` index). Intern names, types, and spans. Fingerprint comparison becomes index comparison.

## 6. `RefCell` prevents future parallelism
`QueryStorage` and `AccumulatorStore` use `RefCell`, which is `!Sync`. Retrofitting `Send + Sync` later requires changing bounds across the entire crate.

**Fix:** Replace `RefCell` with `parking_lot::RwLock` (or `elsa::FrozenMap`) now. Near-zero overhead when uncontended, but makes the future parallelism migration painless. Also add `Send + Sync` to the `Component` blanket impl.

## 7. No cache eviction
For a long-running LSP, query caches grow unboundedly. Not a problem for batch compilation but will be for IDE use.

**Fix (later):** Add LRU eviction or revision-based pruning. Not urgent until LSP integration.

## 8. No query grouping
As the query count grows, there's no way to reason about related queries as a unit (e.g., "all name-res queries" or "clear all inference caches").

**Fix (later):** Add a `QueryGroup` concept for bulk operations and organizational clarity.
