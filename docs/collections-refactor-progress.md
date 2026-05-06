# Collections Refactor — Progress

Tracking the migration plan in `docs/collections-refactor.md`. Items are
grouped by phase. `[x]` = landed; `[ ]` = pending.

---

## Phase 1: Foundation

- [x] Add `CowBox[T]` to `std.memory`
- [x] Add `Slice[T]` protocol with `asSlice()` kernel (called `Slice[T]`,
      not `Seq[T]`; the concrete struct is `ArraySlice[T]`)
- [x] Add `extend Slice[T]` with all read-only methods
- [x] Make `Array[T]` conform to `Slice[T]`
- [x] Fix `ArraySlice`'s broken `isEqual` — element-wise comparison
- [x] Add `Iterable` conformance to `ArraySlice`
- [x] Make `ArraySlice` conform to `Slice[T]`
- [x] Add `Formattable` conformance to `ArraySlice` (conditional on `T: Formattable`)
- [ ] Add `Collection` protocol to `std.core` *(deferred — only needed if/when `Deque` lands)*

---

## Phase 2: Views and Builder

### Views
- [x] `ChunksView[T]` — multi-pass, O(1) `count`, indexed access
- [x] `WindowsView[T]` — multi-pass, O(1) `count`, indexed access
- [x] `ReversedView[T]` — multi-pass, O(1) `count`, indexed access
- [x] `ArraySplitView[T]` (separator-based, `T: Equatable`) — iter + `toArray`
- [x] `ArraySplitWhereView[T]` (predicate-based) — iter + `toArray`
- [x] `ChunksIterator` / `WindowsIterator` moved from `array.ks` into `views.ks`
- [x] New iterators co-located: `SplitIterator`, `SplitWhereIterator`, `ReversedSliceIterator`

### Builder
- [x] `ArrayBuilder[T]` — `not Copyable`, no COW, zero-copy `build()`
- [x] `init()`, `init(capacity:)`, `append(element:)`, `append(contentsOf:)`, `appendFrom[I]`
- [x] `build()`, `clear()`, `count`, `isEmpty`, `capacity`, `deinit`

### Wiring (Slice extension)
- [x] `chunks(of:)` → `ChunksView`
- [x] `windows(of:)` → `WindowsView`
- [x] `reversed()` → `ReversedView`
- [x] `split(separator:)` → `ArraySplitView` (where `T: Equatable`)
- [x] `split(where:)` → `ArraySplitWhereView`
- [x] Old eager `Array.reversed() -> Array[T]`, `Array.chunks(of:) -> ChunksIterator`, `Array.windows(of:) -> WindowsIterator` removed

### Collection-returning transforms (eager)
- [x] `map[U]` on `extend Slice[T]` → `Array[U]`
- [x] `filter` on `extend Slice[T]` → `Array[T]`
- [x] `compactMap[U]` on `extend Slice[T]` → `Array[U]`
- [x] `flatMap[U]` on `extend Slice[T]` → `Array[U]`

### Generalized `collect()`
- [x] `String.init[I](from chars: I) where I: Iterable, I.Item = Char`
- [x] `Array[T].init[I](from:)`, `Set[T].init[I](from:)`, `Dictionary[K,V].init[I](from:)` already exist — confirmed reusable
- [ ] `Iterator.collectInto[C]()` via `Convertible[Self]` — *deferred (parameterized conditional conformance probe pending)*
- [ ] Per-collection `Convertible[I]` conformances (Array, Set, Dictionary, String)

---

## Phase 3: Index Protocol Unification

- [x] **Probe**: `SeqIndex[T]` protocol with generic-method `readSeq[S](from: S) where S: Slice[T]`
- [x] **Probe**: `Int64: SeqIndex[T]` parameterized conditional conformance
- [x] **Probe**: subscript via `extend Slice[T]` works (compiler bug fixed in `c59fc473`)
- [x] Define full `SeqIndex[T]` protocol (`readSeq` / `readSeqChecked` / `readSeqUnchecked` / `writeSeq` / `writeSeqUnchecked`)
- [x] Define `SeqClampable[T]` protocol
- [x] Define `SeqWrappable[T]` protocol
- [x] `Int64: SeqIndex[T]` — full read + write
- [x] `Range[Int64]: SeqIndex[T]` — full read + write
- [x] `ClosedRange[Int64]: SeqIndex[T]` — full read + write
- [x] `Int64: SeqClampable[T]` / `Range[Int64]: SeqClampable[T]`
- [x] `Int64: SeqWrappable[T]`
- [x] Wire subscripts: `(checked:)` on `extend Slice[T]`; `(index:)`, `(unchecked:)`, `(clamped:)`, `(wrapped:)` on `Array[T]` and `extend ArraySlice[T]`
- [x] Remove old `ArrayIndex[T]` / `ArrayClampable[T]` / `ArrayWrappable[T]` from `array.ks`
- [x] Remove old `SliceIndex[T]` / `SliceClampable[T]` / `SliceWrappable[T]` from `pointer.ks`
- [x] Drop probe subscripts (`probe:` / `probeChecked:`) and probe protocol; replaced with full implementation
- [x] Test `seqindex_probe.ks` updated to exercise all subscript variants
- [ ] Add subscripts to views (`ChunksView`, `WindowsView`, `ReversedView`); remove `.at(i)` debt
- [ ] Move write subscripts to `extend Slice[T]` once compiler bug is fixed (subscript set blocks mis-resolve extension `T` as subscript type param `I`)

---

## Phase 4: Sort and Internal Improvements

- [ ] Replace insertion sort with introsort
- [ ] Refactor `Array[T]` to use `CowBox[ArrayStorage[T]]`
- [ ] Refactor `String` to use `CowBox[StringStorage]`
- [ ] Refactor `Dictionary[K, V]` to use `CowBox[DictStorage]`
- [ ] Merge `ArrayIterator[T]` and `ArraySliceIterator[T]` into one type

---

## Phase 5: New Data Structures

- [ ] `Deque[T]`
- [ ] `OrderedDictionary[K, V]`
- [ ] `Heap[T]`
- [ ] `BitSet`
- [ ] `SortedDictionary[K, V]`
- [ ] `SortedSet[T]`
- [ ] Promote `Collection` protocol to `std.core` once non-contiguous types exist

---

## Phase 6: Breaking Changes and Cleanup

- [ ] Remove old `ArrayIndex[T]` / `SliceIndex[T]` protocols and conformances *(folded into Phase 3 if landed there)*
- [ ] Remove `ArrayIterator[T]` (replaced by `ArraySliceIterator[T]`)
- [x] Remove `ChunksIterator[T]` / `WindowsIterator[T]` as direct `Array` return types *(done in Phase 2; structs survive as view internals)*
- [ ] Move `ArraySlice[T]` from `std.memory` to `std.collections` (or re-export from there)
- [ ] Update all stdlib call sites that used the old API surface

---

## Compiler Prerequisites Resolved Along the Way

- [x] `c59fc473` — fixed subscript-resolution shadowing: protocol-extension subscripts no longer hide a concrete type's inherent subscripts when label sets are disjoint. Unblocked Phase 3.

## Known Compiler Bugs

- **Subscript set-block type parameter mis-resolution**: Inside a `set` block on `extend Slice[T]` with a subscript generic parameter `[I]`, the compiler resolves `self.asSlice()` as `ArraySlice[I]` instead of `ArraySlice[T]`. Getter blocks work correctly. Workaround: write subscripts defined on concrete types (`Array[T]`, `extend ArraySlice[T]`) instead of the protocol extension.
