// Dictionary[K, V] - hash map with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable, Addable)
import std.text.(Formattable, FormatOptions, String)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
import std.iter.(Iterator, Iterable)
import std.collections.(DefaultHasher, Array)

// ============================================================================
// BUCKET ENUM
// ============================================================================

/// One slot in the open-addressed hash table backing `Dictionary[K, V, H]`.
///
/// The three cases give the probe loop everything it needs to decide what
/// to do at each step: keep probing, stop, or take the slot. Tombstones
/// (`.Deleted`) preserve probe chains after a key is removed so later
/// lookups for keys that originally collided still find their entries.
/// The cached hash on `.Occupied` lets resizes rehash without re-running
/// the user's `Hash` implementation.
///
/// # Examples
///
/// ```
/// // Internal — not constructed by users. The probe loop reads:
/// match self.buckets.offset(by: i).read() {
///     .Empty       => break,         // miss
///     .Deleted     => continue,      // skip tombstone
///         .Occupied(k, v, _) => ...;
/// }
/// ```
///
/// # Representation
///
/// Tagged enum. `.Occupied` carries the key, value, and 64-bit cached
/// hash inline; `.Empty` and `.Deleted` are nullary tags.
enum Bucket[K, V] {
    /// Slot that has never held an entry; ends a probe chain on lookup.
    case Empty

    /// Tombstone left after a `remove`; probing skips past it but it
    /// does not terminate a lookup.
    case Deleted

    /// Live entry: key, value, and the cached 64-bit hash of the key.
    case Occupied(K, V, UInt64)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Returns the smallest power of two `>= n`, clamped up to a minimum of 8.
///
/// Used to pick the next hash-table capacity. A power of two is required
/// so that `hash mod cap` reduces to `hash and (cap - 1)` and so that
/// the probe sequence visits every slot exactly once. Saturates at the
/// last representable power of two on `Int64` overflow rather than
/// wrapping.
func nextPowerOfTwo(n: Int64) -> Int64 {
    var p: Int64 = Int64(intLiteral: 1);
    while p < n {
        let next = p * Int64(intLiteral: 2);
        // Check for overflow (since Int64 is signed)
        if next <= p {
            return p
        }
        p = next
    }
    let minCap = Int64(intLiteral: 8);
    if p < minCap { minCap } else { p }
}

// ============================================================================
// DICTIONARY ITERATOR
// ============================================================================

/// Single-pass forward iterator over the `(key, value)` entries of a
/// `Dictionary[K, V, H]`.
///
/// Produced by `Dictionary.iter()`. Walks the bucket array once, skipping
/// `.Empty` and `.Deleted` slots, and yields each occupied entry as a
/// tuple. Iteration order matches bucket layout, which depends on the
/// hash and probe sequence — treat it as unspecified. For key- or
/// value-only views see `KeysIterator` and `ValuesIterator`.
///
/// # Examples
///
/// ```
/// let dict = ["a": 1, "b": 2];
/// var it = dict.iter();
/// it.next();  // Some(("a", 1))   — order is unspecified
/// it.next();  // Some(("b", 2))
/// it.next();  // None
/// ```
///
/// # Representation
///
/// A `(buckets, capacity, index)` triple — pointer to the bucket array,
/// total slots, and the current scan position.
///
/// # Memory Model
///
/// Value type. The pointer aliases dictionary storage; do not retain an
/// iterator across mutations of the source dictionary.
public struct DictionaryIterator[K, V]: Iterator {
    /// Element type yielded by `next()` — a `(key, value)` tuple.
    type Item = (K, V)

    /// Pointer to the source dictionary's bucket array.
    private var buckets: Pointer[Bucket[K, V]]
    /// Total number of slots in the bucket array.
    private var capacity: Int64
    /// Current scan position; advances every `next()` call.
    private var index: Int64

    /// @name From Buckets
    /// Constructs an iterator over a raw bucket pointer of the given
    /// capacity.
    ///
    /// Prefer `Dictionary.iter()` over calling this directly. The
    /// pointer must outlive the iterator.
    ///
    /// # Safety
    ///
    /// `buckets` must point to at least `capacity` initialized
    /// `Bucket[K, V]` slots and remain valid for the iterator's
    /// lifetime.
    init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
        self.index = Int64(intLiteral: 0);
    }

    /// Advances the scan to the next occupied slot and returns its
    /// entry, or `None` when no more remain.
    ///
    /// Skips `.Empty` and `.Deleted` slots silently. Once `None` is
    /// returned the iterator stays exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = ["a": 1].iter();
    /// it.next();  // Some(("a", 1))
    /// it.next();  // None
    /// ```
    public mutating func next() -> (K, V)? {
        while self.index < self.capacity {
            let bucket = self.buckets.offset(by: self.index).read();
            self.index = self.index + Int64(intLiteral: 1);
            match bucket {
                .Occupied(key, value, _) => return .Some((key, value)),
                _ => {}
            }
        }
        .None
    }
}

// ============================================================================
// DICTIONARY STORAGE (Internal)
// ============================================================================

/// Internal `(buckets, len, cap)` storage cell shared by
/// `Dictionary[K, V, H]` instances.
///
/// Wrapped in an `RcBox` by `Dictionary` so copying a dictionary just
/// bumps a refcount; mutations call `makeUnique()` first, which
/// deep-copies via `clone()` here. The bucket array always has
/// power-of-two `cap` (or zero), maintained by `nextPowerOfTwo`.
///
/// # Examples
///
/// ```
/// // Not used directly. Created by Dictionary's initializers.
/// let s = DictionaryStorage(buckets: ptr, len: 0, cap: 8);
/// ```
///
/// # Representation
///
/// Three fields: heap pointer to the bucket array, count of live
/// entries (`len`), and total slots (`cap`). `cap == 0` indicates a
/// null pointer with no allocation.
///
/// # Memory Model
///
/// Owns the bucket buffer; `deinit` deallocates it. Used as the value
/// inside an `RcBox`, providing the refcount that powers COW on the
/// dictionary level.
struct DictionaryStorage[K, V, H]: Cloneable where K: Hash, H: Hasher, H: Defaultable {
    /// Heap pointer to the bucket array; null when `cap == 0`.
    var buckets: Pointer[Bucket[K, V]]
    /// Number of `.Occupied` entries (excludes `.Empty` and `.Deleted`).
    var len: Int64
    /// Total bucket count; always a power of two or zero.
    var cap: Int64

    /// @name From Fields
    /// Constructs a storage cell from raw fields.
    ///
    /// Internal — callers must guarantee `cap` is zero or a power of
    /// two and that `buckets` covers `cap` initialized slots.
    init(buckets buckets: Pointer[Bucket[K, V]], len len: Int64, cap cap: Int64) {
        self.buckets = buckets;
        self.len = len;
        self.cap = cap;
    }

    /// Deep-copies the storage into a fresh allocation.
    ///
    /// Allocates a new bucket array of the same `cap` and copies every
    /// slot — including `.Empty` and `.Deleted` — so the copy preserves
    /// existing probe chains. Empty storage clones to empty storage
    /// without allocating. Panics on allocation failure. This is the
    /// slow half of COW, fired by `Dictionary.makeUnique()` when
    /// storage is shared.
    func clone() -> DictionaryStorage[K, V, H] {
        if self.cap == Int64(intLiteral: 0) {
            return DictionaryStorage(
                buckets: Pointer(raw: lang.ptr_null[Bucket[K, V]]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            )
        }
        let layout = Layout.array[Bucket[K, V]](self.cap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newBuckets = rawPtr.cast[Bucket[K, V]]();
            // Copy buckets
            for i in 0..<self.cap {
                newBuckets.offset(by: i).write(self.buckets.offset(by: i).read());
            }
            DictionaryStorage(
                buckets: newBuckets,
                len: self.len,
                cap: self.cap
            )
        } else {
            lang.panic("DictionaryStorage clone allocation failed")
        }
    }

    /// Frees the bucket array.
    ///
    /// Runs when the last `RcBox` reference to this storage drops.
    /// Skips deallocation entirely when `cap == 0` (no buffer was
    /// allocated). Bucket payloads are not destructed individually —
    /// `K` and `V` are treated as trivially droppable here.
    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[Bucket[K, V]](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.buckets.asRaw(), layout)
        }
    }
}

// ============================================================================
// DICTIONARY
// ============================================================================

/// An unordered hash map keyed by any `K: Hash`, parameterized over the
/// hasher type `H` (defaults to `DefaultHasher`).
///
/// Uses open addressing with linear probing and a 75% load-factor
/// threshold for resizes; capacity always grows to the next power of
/// two. Storage is reference-counted with copy-on-write, so copying a
/// `Dictionary` is O(1) and only the next mutation pays for the deep
/// clone. Iteration order is unspecified and may change between
/// versions or after any mutation. For ordered alternatives consider
/// keeping an ordered key list separately; for set-only behavior see
/// `Set`.
///
/// # Examples
///
/// ```
/// var ages: [String: Int64] = [:];
/// ages("Alice") = 30;
/// ages("Bob")   = 25;
///
/// ages("Alice");                // Some(30)
/// ages("Carol", default: 0);    // 0
///
/// for (name, age) in ages.iter() { ... }
/// let sum = ages.values.iter().sum();
/// ```
///
/// # Hashing
///
/// The hash for each key is cached in its bucket so resizes don't
/// recompute it. Replacing the hasher (`H`) lets you swap in
/// `SipHasher`, `FxHasher`, etc.; the default is `DefaultHasher` and
/// resolves through the `[K: V]` shorthand.
///
/// # Capacity & Reallocation
///
/// `count` is live entries; `capacity` is total slots. The table
/// resizes (doubling capacity, starting from 8) once `count` reaches
/// 75% of `capacity`. Use `reserveCapacity(...)` to pre-grow and
/// `shrinkToFit()` to release excess.
///
/// # Representation
///
/// One field: an `RcBox[DictionaryStorage[K, V, H]]` holding
/// `(buckets, len, cap)` over a heap bucket array.
///
/// # Memory Model
///
/// Reference-counted storage with copy-on-write *value* semantics.
/// Copying a `Dictionary` is O(1) and shares the bucket array; the
/// next mutation on a shared dictionary triggers `makeUnique()`,
/// which deep-clones via `DictionaryStorage.clone()` so the mutation
/// is invisible to other copies.
///
/// # Guarantees
///
/// - Every key satisfies `K: Hash`. The cached hash is computed once
///   per insert and reused on resize.
/// - `count <= capacity * 3 / 4` after every mutation (the resize
///   threshold).
/// - Removing a key leaves a `.Deleted` tombstone; lookups still
///   work but tombstones reduce effective capacity until the next
///   resize.
/// - Iteration order is **not** specified.
public struct Dictionary[K, V, H = DefaultHasher]: Iterable, Cloneable where K: Hash, H: Hasher, H: Defaultable {
    /// `Iterable` element type — a `(key, value)` tuple.
    type Item = (K, V)
    /// Concrete iterator type returned by `iter()`.
    type Iter = DictionaryIterator[K, V]

    /// Refcounted storage cell. Sharing this between `Dictionary`
    /// copies enables COW.
    private var storage: RcBox[DictionaryStorage[K, V, H]]

    /// Returns the bucket-array pointer from storage. Internal helper.
    private func buckets() -> Pointer[Bucket[K, V]] { self.storage.getValue().buckets }
    /// Returns the live-entry count from storage. Internal helper.
    private func len() -> Int64 { self.storage.getValue().len }
    /// Returns the total bucket capacity from storage. Internal helper.
    private func cap() -> Int64 { self.storage.getValue().cap }

    /// Ensures the storage is uniquely owned, deep-copying it if shared.
    ///
    /// COW write barrier: every mutating method calls this before
    /// touching the bucket array, so writes never leak into other
    /// `Dictionary` copies that share the same `RcBox`. No-op when
    /// this is the only reference.
    private mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = RcBox(self.storage.getValue().clone())
        }
    }

    /// @name From Storage
    /// Wraps an existing storage box in a new `Dictionary`. Used by
    /// `clone()` and other helpers that already have an `RcBox` in
    /// hand.
    private init(storage storage: RcBox[DictionaryStorage[K, V, H]]) {
        self.storage = storage;
    }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// @name Empty
    /// Creates an empty dictionary with no allocation.
    ///
    /// Capacity starts at zero; the first insert allocates the smallest
    /// bucket array (currently 8 slots). For pre-sized creation use
    /// `init(capacity:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Dictionary[String, Int64]();
    /// d.count;     // 0
    /// d.capacity;  // 0
    /// ```
    public init() {
        self.storage = RcBox(DictionaryStorage(
            buckets: Pointer(raw: lang.ptr_null[Bucket[K, V]]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    /// @name With Capacity
    /// Creates an empty dictionary sized to hold at least the requested
    /// number of entries without resizing.
    ///
    /// The actual allocated capacity is the next power of two `>= capacity`
    /// (minimum 8). A non-positive `capacity` behaves like `init()` (no
    /// allocation). Panics on allocation failure.
    ///
    /// # Examples
    ///
    /// ```
    /// var d = Dictionary[String, Int64](capacity: 100);
    /// d.capacity;   // 128 (next power of two)
    /// d.count;      // 0
    /// ```
    public init(capacity capacity: Int64) {
        let actualCap = nextPowerOfTwo(capacity);
        if actualCap > Int64(intLiteral: 0) {
            let layout = Layout.array[Bucket[K, V]](actualCap);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newBuckets = rawPtr.cast[Bucket[K, V]]();
                // Initialize all buckets as empty
                for i in 0..<actualCap {
                    newBuckets.offset(by: i).write(.Empty);
                }
                self.storage = RcBox(DictionaryStorage(
                    buckets: newBuckets,
                    len: Int64(intLiteral: 0),
                    cap: actualCap
                ))
            } else {
                lang.panic("Dictionary allocation failed")
            }
        } else {
            self.storage = RcBox(DictionaryStorage(
                buckets: Pointer(raw: lang.ptr_null[Bucket[K, V]]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    /// @name From Pairs
    /// Creates a dictionary by inserting every `(key, value)` pair
    /// produced by an iterable.
    ///
    /// Last write wins for duplicate keys. For a panic-on-duplicate
    /// variant use `init(uniqueKeysWithValues:)`. Capacity grows
    /// geometrically as inserts arrive — for sized sources, follow up
    /// with `shrinkToFit()` if memory matters.
    ///
    /// # Examples
    ///
    /// ```
    /// let pairs = [("a", 1), ("b", 2)];
    /// let dict = Dictionary(from: pairs);              // ["a": 1, "b": 2]
    /// let dups = Dictionary(from: [("a", 1), ("a", 2)]);  // ["a": 2] — later wins
    /// ```
    public init[I](from pairs: I) where I: Iterable, I.Item = (K, V) {
        self.init();
        var iter = pairs.iter();
        while let .Some(pair) = iter.next() {
            let _ = self.insert( pair.0, pair.1);
        }
    }

    /// @name Grouping
    /// Buckets each element of an iterable into an array under the key
    /// derived from `keyFunc`.
    ///
    /// The value type is constrained to `Array[E]`: each bucket
    /// accumulates the elements that mapped to it, in insertion order
    /// within that bucket. Useful for building "index-by" tables from a
    /// flat collection. The `keyFunc` runs once per element.
    ///
    /// # Examples
    ///
    /// ```
    /// let words = ["apple", "apricot", "banana", "blueberry"];
    /// let grouped = Dictionary(grouping: words, by: { (w) in w.chars.first().unwrap() });
    /// // ["a": ["apple", "apricot"], "b": ["banana", "blueberry"]]
    ///
    /// let nums = [1, 2, 3, 4, 5];
    /// let parity = Dictionary(grouping: nums, by: { (n) in n % 2 });
    /// // [0: [2, 4], 1: [1, 3, 5]]
    /// ```
    public init[I, E](grouping elements: I, by keyFunc: (E) -> K)
        where I: Iterable, I.Item = E, V = Array[E]
    {
        self.init();
        var iter = elements.iter();
        while let .Some(element) = iter.next() {
            let key = keyFunc(element);
            let maybeArray: Array[E]? = self(key);
            match maybeArray {
                .Some(arr) => {
                    var newArr: Array[E] = arr;
                    newArr.append( element);
                    let _ = self.insert( key, newArr);
                },
                .None => {
                    var arr = Array[E]();
                    arr.append( element);
                    let _ = self.insert( key, arr);
                }
            }
        }
    }

    /// @name Unique Keys
    /// Creates a dictionary from key-value pairs, panicking on any
    /// duplicate key.
    ///
    /// Use this when duplicate keys would indicate a bug in upstream
    /// data; for last-write-wins semantics use `init(from:)`. Each pair
    /// triggers a `contains` check before insertion, so it's slower
    /// than `init(from:)` for large inputs.
    ///
    /// # Errors
    ///
    /// Panics with `"Dictionary(uniqueKeysWithValues:): duplicate key"`
    /// the first time `pairs` yields a key already in the dictionary.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = Dictionary(uniqueKeysWithValues: [("a", 1), ("b", 2)]);
    /// Dictionary(uniqueKeysWithValues: [("a", 1), ("a", 2)]);
    /// // PANIC: Dictionary(uniqueKeysWithValues:): duplicate key
    /// ```
    public init[I](uniqueKeysWithValues pairs: I) where I: Iterable, I.Item = (K, V) {
        self.init();
        var iter = pairs.iter();
        while let .Some(pair) = iter.next() {
            if self.contains( pair.0) {
                lang.panic("Dictionary(uniqueKeysWithValues:): duplicate key")
            }
            let _ = self.insert( pair.0, pair.1);
        }
    }

    // ========================================================================
    // HASHING (Internal)
    // ========================================================================

    /// Computes the 64-bit hash for `key` using a fresh `H` instance.
    ///
    /// Calls `key.hash(into:)` then `hasher.finish()`. Result is cached
    /// in `.Occupied` buckets so resizes don't recompute it.
    private func hashKey(key: K) -> UInt64 {
        var hasher = H();
        key.hash(into: hasher);
        hasher.finish()
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Number of live (`.Occupied`) entries. Read-only; O(1).
    ///
    /// Excludes tombstones — `count` only reflects what
    /// `iter()`/`contains(...)` would see.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].count;  // 2
    /// [:].count;               // 0
    /// ```
    public var count: Int64 { get { self.len() } }

    /// Total slots in the bucket array — always `>= count`. Read-only.
    ///
    /// Resizes (doubling) trigger when `count` reaches 75% of
    /// `capacity`. Tombstones count against the threshold even though
    /// they don't count toward `count`. The actual value after
    /// `init(capacity:)` rounds up to the next power of two.
    ///
    /// # Examples
    ///
    /// ```
    /// let d = Dictionary[String, Int64](capacity: 100);
    /// d.capacity;  // 128
    /// ```
    public var capacity: Int64 { get { self.cap() } }

    /// `true` when the dictionary holds no live entries; equivalent to
    /// `count == 0`.
    ///
    /// Reads more naturally than the comparison.
    ///
    /// # Examples
    ///
    /// ```
    /// [:].isEmpty;           // true
    /// ["a": 1].isEmpty;      // false
    /// ```
    public var isEmpty: Bool { get { self.len() == Int64(intLiteral: 0) } }

    /// Lazy view of the dictionary's keys, iterable in unspecified
    /// order.
    ///
    /// Constructing the view is O(1) — it shares the bucket pointer
    /// and skips empty/deleted slots during iteration. The view is
    /// invalidated by any mutation that may reallocate (insertion past
    /// the load threshold, `reserveCapacity`, `shrinkToFit`).
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2, "c": 3];
    /// for key in dict.keys { print(key) }
    /// let keyArray = Array(from: dict.keys);
    /// ```
    public var keys: KeysView[K, V] { get { KeysView(buckets: self.getBuckets(), capacity: self.cap()) } }

    /// Lazy view of the dictionary's values, iterable in unspecified
    /// order.
    ///
    /// Same iteration order as `keys` — the two views walk the
    /// buckets in lockstep, so `zip(dict.keys, dict.values)` yields
    /// pairs equivalent to `dict.iter()`. Invalidated by any
    /// mutation that may reallocate.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2, "c": 3];
    /// for value in dict.values { print(value) }
    /// let sum = dict.values.iter().sum();  // 6
    /// ```
    public var values: ValuesView[K, V] { get { ValuesView(buckets: self.getBuckets(), capacity: self.cap()) } }

    // ========================================================================
    // SUBSCRIPTS
    // ========================================================================

    /// @name Lookup
    /// Reads the value for `key` (or `None` if absent), or assigns
    /// to insert/remove the entry.
    ///
    /// The assignment form treats `Some(v)` as insert/update and
    /// `None` as delete — so `dict(k) = None` is the inline form of
    /// `dict.remove(k)`. For a non-`Optional` getter use
    /// `dict(key, default: ...)` or `dict(unwrap: key)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2];
    /// dict("a");           // Some(1)
    /// dict("z");           // None
    /// dict("c") = 3;       // inserts "c": 3
    /// dict("a") = None;    // removes "a"
    /// ```
    public subscript(key: K) -> V? {
        get {
            let maybeIndex = self.findEntry(key);
            if let .Some(index) = maybeIndex {
                let bucket = self.buckets().offset(by: index).read();
                match bucket {
                    .Occupied(_, value, _) => .Some(value),
                    _ => .None
                }
            } else {
                .None
            }
        }
        set {
            match newValue {
                .Some(value) => {
                    let _ = self.insert( key, value);
                },
                .None => {
                    let _ = self.remove( key);
                }
            }
        }
    }

    /// @name With Default
    /// Reads the value for `key`, falling back to `defaultValue` when
    /// the key is absent.
    ///
    /// Read-only and *non-inserting* — the default value is returned
    /// but never stored. To upsert with a default, use `upsert(...)`
    /// or `update(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2];
    /// dict("a", default: 0);  // 1
    /// dict("z", default: 0);  // 0
    /// dict("z");              // still None — default wasn't stored
    /// ```
    public subscript(key: K, default defaultValue: V) -> V {
        get {
            let maybeValue = self(key);
            match maybeValue {
                .Some(v) => v,
                .None => defaultValue
            }
        }
    }

    /// @name Unwrap
    /// Reads or writes the value for `key`, panicking on the read
    /// when the key is absent.
    ///
    /// Use when you've already verified the key exists (or when its
    /// absence indicates a bug). The setter is equivalent to
    /// `insert(key, newValue)` and never panics. For a non-panicking
    /// read use `dict(key)` or `dict(key, default: ...)`.
    ///
    /// # Errors
    ///
    /// Read panics with
    /// `"Dictionary subscript(unwrap:): key not found"` when the key
    /// is absent.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2];
    /// dict(unwrap: "a");  // 1
    /// dict(unwrap: "z");  // PANIC: key not found
    /// ```
    public subscript(unwrap key: K) -> V {
        get {
            let maybeValue = self(key);
            match maybeValue {
                .Some(v) => v,
                .None => lang.panic("Dictionary subscript(unwrap:): key not found")
            }
        }
        set {
            let _ = self.insert( key, newValue);
        }
    }

    // ========================================================================
    // INTERNAL LOOKUP
    // ========================================================================

    /// Locates the bucket index storing `key`, or `None` if absent.
    ///
    /// Probes linearly from the hash-anchored start slot, skipping
    /// `.Deleted` tombstones and stopping at the first `.Empty` slot
    /// (which terminates a chain). Returns `None` after a full
    /// wrap-around without finding the key.
    private func findEntry(key: K) -> Int64? {
        let myCap = self.cap();
        if myCap == Int64(intLiteral: 0) {
            return .None
        }

        let hashValue: UInt64 = self.hashKey(key);
        let capU: UInt64 = UInt64(from: myCap);
        let mod: UInt64 = hashValue.modulo(capU);
        var index: Int64 = Int64(from: mod);
        let myBuckets = self.buckets();
        var i: Int64 = Int64(intLiteral: 0);

        while i < myCap {
            let bucket = myBuckets.offset(by: index).read();
            match bucket {
                .Empty => return .None,
                .Occupied(k, _, _) => {
                    if k.equals(key) {
                        return .Some(index)
                    }
                },
                .Deleted => {}
            }
            // Linear probing
            index = (index + Int64(intLiteral: 1)) % myCap;
            i = i + Int64(intLiteral: 1)
        }
        .None
    }

    /// Returns the index of the first non-`.Occupied` slot reachable
    /// from `hashValue`'s anchor.
    ///
    /// Used by `insert` after `findEntry` has confirmed the key is
    /// new. Reuses tombstones, which keeps the bucket array compact
    /// after many removes followed by inserts.
    private func findEmptySlot(hashValue: UInt64) -> Int64? {
        let myCap = self.cap();
        if myCap == Int64(intLiteral: 0) {
            return .None
        }

        let capU: UInt64 = UInt64(from: myCap);
        let mod: UInt64 = hashValue.modulo(capU);
        var index: Int64 = Int64(from: mod);
        let myBuckets = self.buckets();
        var i: Int64 = Int64(intLiteral: 0);

        while i < myCap {
            let bucket = myBuckets.offset(by: index).read();
            match bucket {
                .Occupied(_, _, _) => {},
                _ => return .Some(index)
            }
            // Linear probing
            index = (index + Int64(intLiteral: 1)) % myCap;
            i = i + Int64(intLiteral: 1)
        }
        .None
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Triggers `resize()` if `count >= 75% * capacity` (or capacity is
    /// zero).
    ///
    /// Called by `insert` before placing a new entry. Resizing
    /// scattering tombstones is desirable — `resize()` rebuilds the
    /// table without them.
    private mutating func ensureCapacity() {
        let myCap = self.cap();
        let myLen = self.len();
        let threshold = myCap * Int64(intLiteral: 3) / Int64(intLiteral: 4);
        if myLen >= threshold or myCap == Int64(intLiteral: 0) {
            self.resize()
        }
    }

    /// Doubles the bucket array (or jumps from 0 to 8) and rehashes
    /// every live entry.
    ///
    /// Tombstones are dropped during the rebuild, so resizes also
    /// reclaim space wasted by previous removes. Triggers COW.
    /// Panics on allocation failure.
    private mutating func resize() {
        self.makeUnique();
        let s = self.storage.getValue();
        let newCap: Int64 = if s.cap == Int64(intLiteral: 0) {
            Int64(intLiteral: 8)
        } else {
            s.cap * Int64(intLiteral: 2)
        };

        let oldBuckets = s.buckets;
        let oldCap = s.cap;

        // Allocate new table
        let layout = Layout.array[Bucket[K, V]](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newBuckets = rawPtr.cast[Bucket[K, V]]();

            // Initialize new buckets as empty
            for i in 0..<newCap {
                newBuckets.offset(by: i).write(.Empty);
            }

            // Copy occupied buckets
            var newLen: Int64 = Int64(intLiteral: 0);
            for i in 0..<oldCap {
                let bucket = oldBuckets.offset(by: i).read();
                match bucket {
                    .Occupied(key, value, hashValue) => {
                        // Find empty slot in new table using hashing and linear probing
                        let newCapU: UInt64 = UInt64(from: newCap);
                        let mod: UInt64 = hashValue.modulo(newCapU);
                        var slotIndex: Int64 = Int64(from: mod);
                        var foundSlot: Bool = false;
                        while foundSlot == false {
                            let slotBucket = newBuckets.offset(by: slotIndex).read();
                            match slotBucket {
                                .Empty => {
                                    newBuckets.offset(by: slotIndex).write(.Occupied(key, value, hashValue));
                                    newLen = newLen + Int64(intLiteral: 1);
                                    foundSlot = true
                                },
                                _ => slotIndex = (slotIndex + Int64(intLiteral: 1)) % newCap
                            }
                        }
                    },
                    _ => {}
                }
            }

            // Free old table
            if oldCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[Bucket[K, V]](oldCap);
                allocator.deallocate(oldBuckets.asRaw(), oldLayout)
            }

            self.storage.setValue(DictionaryStorage(
                buckets: newBuckets,
                len: newLen,
                cap: newCap
            ))
        } else {
            lang.panic("Dictionary resize failed")
        }
    }

    /// Rebuilds the bucket array at the exact `newCap` capacity.
    ///
    /// Used by `reserveCapacity` and `shrinkToFit`. The caller must
    /// ensure `newCap` is large enough to hold the current `count`
    /// without exceeding the load factor; this routine itself does
    /// not check. Triggers COW. Panics on allocation failure.
    private mutating func resizeToCapacity(newCap: Int64) {
        self.makeUnique();
        let s = self.storage.getValue();
        let oldBuckets = s.buckets;
        let oldCap = s.cap;

        // Allocate new table
        let layout = Layout.array[Bucket[K, V]](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newBuckets = rawPtr.cast[Bucket[K, V]]();

            // Initialize new buckets as empty
            for i in 0..<newCap {
                newBuckets.offset(by: i).write(.Empty);
            }

            // Copy occupied buckets
            var newLen: Int64 = Int64(intLiteral: 0);
            for i in 0..<oldCap {
                let bucket = oldBuckets.offset(by: i).read();
                match bucket {
                    .Occupied(key, value, hashValue) => {
                        // Find empty slot in new table using hashing and linear probing
                        let newCapU: UInt64 = UInt64(from: newCap);
                        let mod: UInt64 = hashValue.modulo(newCapU);
                        var slotIndex: Int64 = Int64(from: mod);
                        var foundSlot: Bool = false;
                        while foundSlot == false {
                            let slotBucket = newBuckets.offset(by: slotIndex).read();
                            match slotBucket {
                                .Empty => {
                                    newBuckets.offset(by: slotIndex).write(.Occupied(key, value, hashValue));
                                    newLen = newLen + Int64(intLiteral: 1);
                                    foundSlot = true
                                },
                                _ => slotIndex = (slotIndex + Int64(intLiteral: 1)) % newCap
                            }
                        }
                    },
                    _ => {}
                }
            }

            // Free old table
            if oldCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[Bucket[K, V]](oldCap);
                allocator.deallocate(oldBuckets.asRaw(), oldLayout)
            }

            self.storage.setValue(DictionaryStorage(
                buckets: newBuckets,
                len: newLen,
                cap: newCap
            ))
        } else {
            lang.panic("Dictionary resize failed")
        }
    }

    // ========================================================================
    // KEY LOOKUP
    // ========================================================================

    /// `true` if `key` is present in the dictionary.
    ///
    /// Wraps `findEntry`. For value-based search use the `V: Equatable`
    /// extension's `containsValue(value:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].contains(key: "a");  // true
    /// ["a": 1, "b": 2].contains(key: "z");  // false
    /// ```
    public func contains(key: K) -> Bool {
        self.findEntry(key).isSome()
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Inserts `(key, value)`, replacing any existing entry for `key`,
    /// and returns the old value (or `None`) on update.
    ///
    /// Triggers `ensureCapacity()` and may resize before the insert
    /// lands. The cached hash is computed once here. For
    /// transform-based updates see `update(...)` and `upsert(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1];
    /// dict.insert(key: "b", value: 2);  // None;     dict = ["a": 1, "b": 2]
    /// dict.insert(key: "a", value: 9);  // Some(1);  dict = ["a": 9, "b": 2]
    /// ```
    public mutating func insert(key: K, value: V) -> V? {
        let hashValue = self.hashKey(key);
        // Check if key already exists
        let maybeIndex = self.findEntry(key);
        if let .Some(index) = maybeIndex {
            self.makeUnique();
            let myBuckets = self.buckets();
            let oldBucket = myBuckets.offset(by: index).read();
            let oldValue: V? = match oldBucket {
                .Occupied(_, v, _) => .Some(v),
                _ => .None
            };
            myBuckets.offset(by: index).write(.Occupied(key, value, hashValue));
            return oldValue
        }

        // Need to insert - ensure capacity first
        self.ensureCapacity();
        self.makeUnique();

        // Find empty slot
        let maybeSlot = self.findEmptySlot(hashValue);
        if let .Some(slotIndex) = maybeSlot {
            var s = self.storage.getValue();
            s.buckets.offset(by: slotIndex).write(.Occupied(key, value, hashValue));
            s.len = s.len + Int64(intLiteral: 1);
            self.storage.setValue(s)
        } else {
            lang.panic("Dictionary insert failed - no empty slot")
        }

        .None
    }

    /// Removes `key` and returns its value, or `None` if absent.
    ///
    /// Replaces the bucket with a `.Deleted` tombstone so existing
    /// probe chains stay intact. Tombstones are reclaimed by the next
    /// resize. Triggers COW only when an entry is actually removed.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2];
    /// dict.remove(key: "a");  // Some(1); dict = ["b": 2]
    /// dict.remove(key: "z");  // None;    dict unchanged
    /// ```
    public mutating func remove(key: K) -> V? {
        let maybeIndex = self.findEntry(key);

        if let .Some(index) = maybeIndex {
            self.makeUnique();
            var s = self.storage.getValue();
            let bucket = s.buckets.offset(by: index).read();
            let removedValue: V? = match bucket {
                .Occupied(_, v, _) => .Some(v),
                _ => .None
            };

            // Mark as deleted (tombstone)
            s.buckets.offset(by: index).write(.Deleted);
            s.len = s.len - Int64(intLiteral: 1);
            self.storage.setValue(s);

            return removedValue
        }

        .None
    }

    /// Removes every entry, leaving the bucket array allocated and
    /// reset to all-`.Empty`.
    ///
    /// O(capacity). The buffer is kept so subsequent inserts don't
    /// reallocate; follow with `shrinkToFit()` to release it.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2];
    /// dict.clear();    // dict = [:]
    /// dict.capacity;   // unchanged
    /// ```
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        for i in 0..<s.cap {
            s.buckets.offset(by: i).write(.Empty);
        }
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    /// Applies `transform` to the existing value for `key` and writes
    /// the result back; returns whether the key was found.
    ///
    /// No-op when the key is absent — for "update or insert" semantics
    /// use `upsert(...)`. Internally re-uses `insert(...)`, so the
    /// hash is recomputed.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2];
    /// dict.update(key: "a", with: { (v) in v * 10 });  // true;  dict("a") == Some(10)
    /// dict.update(key: "z", with: { (v) in v * 10 });  // false; dict unchanged
    /// ```
    public mutating func update(key: K, with transform: (V) -> V) -> Bool {
        let maybeValue = self(key);
        match maybeValue {
            .Some(v) => {
                let _ = self.insert( key, transform(v));
                true
            },
            .None => false
        }
    }

    /// Inserts `transform(defaultValue)` for a new key, or
    /// `transform(existing)` for an existing one.
    ///
    /// The classic "increment-or-set-to-1" pattern. `defaultValue` is
    /// passed through `transform` even on the insert path, so the same
    /// closure handles both branches uniformly. For the no-insert
    /// variant see `update(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var counts: [String: Int64] = [:];
    /// counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 });
    /// counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 });
    /// counts("apple");  // Some(2)
    /// ```
    public mutating func upsert(key: K, default defaultValue: V, with transform: (V) -> V) {
        let maybeValue = self(key);
        match maybeValue {
            .Some(v) => {
                let _ = self.insert( key, transform(v));
            },
            .None => {
                let _ = self.insert( key, transform(defaultValue));
            }
        }
    }

    /// Merges every entry of `other` into `self`, calling `combine`
    /// to resolve key collisions.
    ///
    /// `combine(existing, incoming)` is invoked exactly once per
    /// collision — pick one, return both summed, or use `(_, new)` for
    /// last-write-wins. New keys are inserted directly. For a
    /// non-mutating variant use `merging(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var a = ["x": 1, "y": 2];
    /// let b = ["y": 20, "z": 30];
    /// a.merge(b, uniquingKeysWith: { (old, new) in old + new });
    /// // a == ["x": 1, "y": 22, "z": 30]
    /// ```
    public mutating func merge(other: Dictionary[K, V, H], uniquingKeysWith combine: (V, V) -> V) {
        var otherIter = other.iter();
        while let .Some(pair) = otherIter.next() {
            let maybeExisting = self(pair.0);
            match maybeExisting {
                .Some(existing) => {
                    let _ = self.insert( pair.0, combine(existing, pair.1));
                },
                .None => {
                    let _ = self.insert( pair.0, pair.1);
                }
            }
        }
    }

    /// Merges every `(key, value)` pair from an arbitrary iterable into
    /// `self`, calling `combine` on collisions.
    ///
    /// Same semantics as `merge(...)` but accepts any iterable of
    /// pairs — useful for arrays of tuples, generator output, or
    /// streamed sources.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1];
    /// dict.mergeFrom(pairs: [("b", 2), ("c", 3)], uniquingKeysWith: { (_, new) in new });
    /// // dict == ["a": 1, "b": 2, "c": 3]
    /// ```
    public mutating func mergeFrom[I](pairs: I, uniquingKeysWith combine: (V, V) -> V)
        where I: Iterable, I.Item = (K, V)
    {
        var iter = pairs.iter();
        while let .Some(pair) = iter.next() {
            let maybeExisting = self(pair.0);
            match maybeExisting {
                .Some(existing) => {
                    let _ = self.insert( pair.0, combine(existing, pair.1));
                },
                .None => {
                    let _ = self.insert( pair.0, pair.1);
                }
            }
        }
    }

    /// Keeps only entries for which `predicate(key, value)` is true.
    ///
    /// Two-pass implementation: collects keys to remove, then deletes
    /// them. Each removal leaves a tombstone — call `shrinkToFit()`
    /// afterwards if you've removed a large fraction. The mirror is
    /// `removeAll(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2, "c": 3];
    /// dict.retain(matching: { (k, v) in v > 1 });  // ["b": 2, "c": 3]
    /// ```
    public mutating func retain(matching predicate: (K, V) -> Bool) {
        self.makeUnique();
        let myCap = self.cap();
        let myBuckets = self.buckets();
        var keysToRemove = Array[K]();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) == false {
                        keysToRemove.append(key);
                    }
                },
                _ => {}
            }
        }

        var removeIter = keysToRemove.iter();
        while let .Some(key) = removeIter.next() {
            let _ = self.remove( key);
        }
    }

    /// Removes every entry for which `predicate(key, value)` is true.
    ///
    /// Inverse of `retain(matching:)`; implemented as `retain` over
    /// the negated predicate. Same tombstone caveat applies — consider
    /// `shrinkToFit()` after large removals.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = ["a": 1, "b": 2, "c": 3];
    /// dict.removeAll(matching: { (k, v) in v < 2 });  // ["b": 2, "c": 3]
    /// ```
    public mutating func removeAll(matching predicate: (K, V) -> Bool) {
        self.retain(matching: { (k, v) in predicate(k, v) == false })
    }

    /// Grows the bucket array so at least `minimumCapacity` entries
    /// fit without resizing.
    ///
    /// No-op when current capacity already suffices. The actual new
    /// capacity rounds up to the next power of two and accounts for
    /// the 75% load factor (so target = `nextPowerOfTwo(min * 4 / 3)`).
    /// The opposite operation is `shrinkToFit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = Dictionary[String, Int64]();
    /// dict.reserveCapacity(minimumCapacity: 1000);
    /// // No reallocations for the first ~750 inserts.
    /// ```
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        if minimumCapacity <= self.capacity {
            return
        }
        // Calculate target capacity (accounting for load factor)
        let targetCap = nextPowerOfTwo(minimumCapacity * Int64(intLiteral: 4) / Int64(intLiteral: 3));
        self.resizeToCapacity(targetCap)
    }

    /// Reduces capacity to the smallest power of two that still
    /// satisfies the load factor for the current `count`.
    ///
    /// Frees excess memory and reclaims tombstone space (the resize
    /// rebuilds the table without them). Empty dictionaries fall
    /// through to `clear()`. No-op when the table is already at the
    /// minimum acceptable capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// var dict = Dictionary[String, Int64](capacity: 1000);
    /// dict("a") = 1;
    /// dict.shrinkToFit();  // capacity drops from 1024 to 8
    /// ```
    public mutating func shrinkToFit() {
        let currentCount = self.count;
        if currentCount == Int64(intLiteral: 0) {
            self.clear();
            return
        }

        let targetCap = nextPowerOfTwo(currentCount * Int64(intLiteral: 4) / Int64(intLiteral: 3));
        if targetCap < self.capacity {
            self.resizeToCapacity(targetCap)
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns a `DictionaryIterator[K, V]` over the live entries.
    ///
    /// Order is unspecified and may change between mutations. The
    /// iterator borrows the bucket array; do not mutate the
    /// dictionary while iterating. For key- or value-only iteration,
    /// use `keys.iter()` / `values.iter()`.
    ///
    /// # Examples
    ///
    /// ```
    /// for (k, v) in dict.iter() { ... }
    /// let entries = Array(from: dict.iter());
    /// ```
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(buckets: self.buckets(), capacity: self.cap())
    }

    /// Internal accessor exposing the bucket pointer to `KeysView` /
    /// `ValuesView` constructors. Not for outside use.
    fileprivate func getBuckets() -> Pointer[Bucket[K, V]] { self.buckets() }

    // ========================================================================
    // SEARCHING AND PREDICATES
    // ========================================================================

    /// `true` if any entry satisfies `predicate(key, value)`.
    ///
    /// Linear scan; short-circuits on the first match. `false` for
    /// empty dictionaries. The aliased shape `any(satisfy:)` exists
    /// for symmetry with `Array`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 5].contains(matching: { (k, v) in v > 3 });  // true
    /// ["a": 1, "b": 2].contains(matching: { (k, v) in v > 3 });  // false
    /// ```
    public func contains(matching predicate: (K, V) -> Bool) -> Bool {
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) {
                        return true
                    }
                },
                _ => {}
            }
        }
        false
    }

    /// Returns *some* entry satisfying `predicate(key, value)`, or
    /// `None`.
    ///
    /// "First" is determined by bucket order, which is hash-dependent
    /// and unspecified — treat the result as arbitrary among matching
    /// entries. Short-circuits on the first match.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 5, "c": 3];
    /// dict.first(matching: { (k, v) in v > 2 });  // Some entry with v > 2
    /// dict.first(matching: { (k, v) in v > 99 }); // None
    /// ```
    public func first(matching predicate: (K, V) -> Bool) -> (K, V)? {
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) {
                        return .Some((key, value))
                    }
                },
                _ => {}
            }
        }
        .None
    }

    /// `true` when every entry satisfies `predicate(key, value)`
    /// (vacuously true for empty).
    ///
    /// Short-circuits on the first failure. Dual of `any(satisfying:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 2, "b": 4].all(satisfying: { (k, v) in v % 2 == 0 });  // true
    /// ["a": 1, "b": 2].all(satisfying: { (k, v) in v % 2 == 0 });  // false
    /// [:].all(satisfying: { (k, v) in false });                    // true (vacuous)
    /// ```
    public func all(satisfying predicate: (K, V) -> Bool) -> Bool {
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) == false {
                        return false
                    }
                },
                _ => {}
            }
        }
        true
    }

    /// `true` when at least one entry satisfies `predicate(key, value)`.
    ///
    /// Alias for `contains(matching:)` — the two names exist so
    /// predicate-style code reads naturally regardless of context.
    /// Short-circuits on the first match.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 5].any(satisfying: { (k, v) in v > 3 });  // true
    /// [:].any(satisfying: { (k, v) in true });                // false (empty)
    /// ```
    public func any(satisfying predicate: (K, V) -> Bool) -> Bool {
        self.contains(matching: predicate)
    }

    /// Returns the number of entries for which
    /// `predicate(key, value)` is true.
    ///
    /// Linear scan, no short-circuit. For just a presence check use
    /// `any(satisfying:)`; for a yes/no on every entry,
    /// `all(satisfying:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2, "c": 3].countItems(matching: { (k, v) in v > 1 });  // 2
    /// [:].countItems(matching: { (k, v) in true });                        // 0
    /// ```
    public func countItems(matching predicate: (K, V) -> Bool) -> Int64 {
        var result: Int64 = Int64(intLiteral: 0);
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) {
                        result = result + Int64(intLiteral: 1);
                    }
                },
                _ => {}
            }
        }
        result
    }

    // ========================================================================
    // TRANSFORMATIONS
    // ========================================================================

    /// Returns a new dictionary with each value run through `transform`,
    /// keys unchanged.
    ///
    /// Pre-sized to `self.capacity` so the first build avoids
    /// resizing. The result's value type can change (`V → U`); for a
    /// version that drops `None` results see `compactMapValues(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2];
    /// let doubled = dict.mapValues(transform: { (v) in v * 2 });
    /// // ["a": 2, "b": 4]
    /// ```
    public func mapValues[U](transform: (V) -> U) -> Dictionary[K, U, H] {
        var result = Dictionary[K, U, H](capacity: self.capacity);
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    let _ = result.insert( key, transform(value));
                },
                _ => {}
            }
        }
        result
    }

    /// Returns a new dictionary with each value run through `transform`;
    /// entries whose `transform(value)` is `None` are dropped.
    ///
    /// Useful for parse-or-skip patterns. The result is unsized at
    /// construction (since the final count isn't known until the
    /// pass completes); for fixed transforms that always succeed,
    /// `mapValues(...)` avoids the allocation policy difference.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": "1", "b": "two", "c": "3"];
    /// let parsed = dict.compactMapValues(transform: { (s) in Int64.parse(s) });
    /// // ["a": 1, "c": 3] — "two" failed to parse
    /// ```
    public func compactMapValues[U](transform: (V) -> U?) -> Dictionary[K, U, H] {
        var result = Dictionary[K, U, H]();
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if let .Some(transformed) = transform(value) {
                        let _ = result.insert( key, transformed);
                    }
                },
                _ => {}
            }
        }
        result
    }

    /// Returns a new dictionary containing only entries for which
    /// `predicate(key, value)` is true.
    ///
    /// Non-mutating mirror of `retain(matching:)`. Allocates a fresh
    /// dictionary; for in-place filtering use `retain` or
    /// `removeAll(matching:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let dict = ["a": 1, "b": 2, "c": 3];
    /// let big = dict.filter(matching: { (k, v) in v > 1 });  // ["b": 2, "c": 3]
    /// ```
    public func filter(matching predicate: (K, V) -> Bool) -> Dictionary[K, V, H] {
        var result = Dictionary[K, V, H]();
        let myCap = self.cap();
        let myBuckets = self.buckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if predicate(key, value) {
                        let _ = result.insert( key, value);
                    }
                },
                _ => {}
            }
        }
        result
    }

    /// Returns a new dictionary that is `self` merged with `other`,
    /// resolving collisions via `combine`.
    ///
    /// Non-mutating mirror of `merge(...)`. Internally clones via COW
    /// (cheap until the next mutation) and merges into the copy.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = ["x": 1, "y": 2];
    /// let b = ["y": 20, "z": 30];
    /// let merged = a.merging(other: b, uniquingKeysWith: { (_, new) in new });
    /// // merged == ["x": 1, "y": 20, "z": 30]
    /// // a is unchanged
    /// ```
    public func merging(other: Dictionary[K, V, H], uniquingKeysWith combine: (V, V) -> V)
        -> Dictionary[K, V, H]
    {
        var result = self.clone();
        result.merge( other, uniquingKeysWith: combine);
        result
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Returns a `Dictionary` sharing the same storage; the deep copy
    /// is deferred until either side mutates.
    ///
    /// O(1) — just bumps the storage `RcBox`'s refcount. The first
    /// mutation on either side triggers `makeUnique()`, which
    /// deep-clones via `DictionaryStorage.clone()`. For an immediate
    /// deep copy use `deepClone()` (defined in the unconditional
    /// extension below).
    ///
    /// # Examples
    ///
    /// ```
    /// let a: [String: Int64] = ["x": 1];
    /// var b = a.clone();  // O(1), shares storage
    /// b("y") = 2;         // b deep-copies here; a is unchanged
    /// ```
    public func clone() -> Dictionary[K, V, H] {
        Dictionary(storage: self.storage.clone())
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// `Equatable` conformance for dictionaries whose values are themselves
/// `Equatable`.
extend Dictionary[K, V, H]: Equatable where K: Hash, V: Equatable, H: Hasher, H: Defaultable {
    /// Order-independent equality: dictionaries are equal iff they have
    /// the same `count` and every key in `self` is present in `other`
    /// with an equal value.
    ///
    /// Short-circuits on the first mismatch. Insertion order does not
    /// matter — only the multiset of `(key, value)` pairs does.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].equals(other: ["b": 2, "a": 1]);  // true
    /// ["a": 1].equals(other: ["a": 2]);                  // false
    /// ["a": 1].equals(other: [:]);                       // false
    /// ```
    public func equals(other: Dictionary[K, V, H]) -> Bool {
        let selfCount = self.count;
        let otherCount = other.count;
        if selfCount != otherCount {
            return false
        }

        // Check all entries in self exist in other with same value
        let selfCap = self.capacity;
        let selfBuckets = self.getBuckets();
        for i in 0..<selfCap {
            let bucket = selfBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    let otherValue = other(key);
                    if let .Some(v) = otherValue {
                        if value.equals(v) == false {
                            return false
                        }
                    } else {
                        return false
                    }
                },
                _ => {}
            }
        }
        true
    }
}

/// Value-based search operations available when `V: Equatable`.
extend Dictionary[K, V, H] where K: Hash, V: Equatable, H: Hasher, H: Defaultable {

    /// `true` if any entry's value equals `value`.
    ///
    /// O(capacity) — every bucket is inspected because the dictionary
    /// is keyed on `K`, not `V`. For `O(1)` checks against a small
    /// set of values, build a `Set[V]` instead.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].containsValue(value: 2);  // true
    /// ["a": 1, "b": 2].containsValue(value: 5);  // false
    /// ```
    public func containsValue(value: V) -> Bool {
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(_, v, _) => {
                    if v.equals(value) {
                        return true
                    }
                },
                _ => {}
            }
        }
        false
    }

    /// Returns *some* key mapping to `value`, or `None`.
    ///
    /// O(capacity); short-circuits on the first match. "First" is
    /// determined by bucket order and is unspecified — for an
    /// exhaustive list use `allKeys(forValue:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].firstKey(forValue: 2);  // Some("b")
    /// ["a": 1, "b": 2].firstKey(forValue: 5);  // None
    /// ```
    public func firstKey(forValue value: V) -> K? {
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(k, v, _) => {
                    if v.equals(value) {
                        return .Some(k)
                    }
                },
                _ => {}
            }
        }
        .None
    }

    /// Returns every key whose value equals `value`.
    ///
    /// O(capacity), allocates an `Array[K]`. Result order matches
    /// bucket layout and is therefore unspecified. Empty array if no
    /// matches.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2, "c": 1].allKeys(forValue: 1);  // ["a", "c"]  — order unspecified
    /// ["a": 1].allKeys(forValue: 99);                  // []
    /// ```
    public func allKeys(forValue value: V) -> Array[K] {
        var result = Array[K]();
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(k, v, _) => {
                    if v.equals(value) {
                        result.append(k);
                    }
                },
                _ => {}
            }
        }
        result
    }
}

/// `Formattable` conformance — renders a dictionary as
/// `"{k1: v1, k2: v2}"` when both `K` and `V` are `Formattable`.
///
/// Drives string interpolation. Empty dictionaries render as `"{}"`.
/// Entry order in the output reflects bucket order and is unspecified.
extend Dictionary[K, V, H]: Formattable where K: Hash, K: Formattable, V: Formattable, H: Hasher, H: Defaultable {
    /// Renders the dictionary as `"{" + entries.joined(", ") + "}"`,
    /// passing `options` to each key and value's `format`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2].format();  // "{a: 1, b: 2}"  — order unspecified
    /// Dictionary[String, Int64]().format();  // "{}"
    /// "\{["a": 1, "b": 2]}";      // "{a: 1, b: 2}"  via interpolation
    /// ```
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var result = "{";
        var first = true;
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    if first {
                        first = false;
                    } else {
                        result = result + ", ";
                    }
                    result = result + key.format(options) + ": " + value.format(options);
                },
                _ => {}
            }
        }
        result + "}"
    }
}

/// Eager-copy variant of `clone()` for callers that don't want to
/// inherit the COW share with the source.
extend Dictionary[K, V, H] where K: Hash, H: Hasher, H: Defaultable {

    /// Returns a fully-detached copy of the dictionary, with no shared
    /// storage.
    ///
    /// Walks every bucket and re-inserts the live entries into a
    /// freshly-sized table. Use over `clone()` when you specifically
    /// want to avoid the lazy COW share — for example, before passing
    /// the copy to another thread or system that might race with
    /// further mutations.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = ["x": [1, 2, 3]];
    /// let b = a.deepClone();  // fully independent copy
    /// ```
    public func deepClone() -> Dictionary[K, V, H] {
        var result = Dictionary[K, V, H](capacity: self.capacity);
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    let _ = result.insert( key, value);
                },
                _ => {}
            }
        }
        result
    }
}

/// Aggregation available when the value type forms an `Addable`
/// monoid (`V + V = V` with a `Defaultable` zero).
extend Dictionary[K, V, H] where K: Hash, V: Addable, V.Output = V, V: Defaultable, H: Hasher, H: Defaultable {

    /// Returns the sum of every value, starting from `V()` (the
    /// default-constructed zero).
    ///
    /// Empty dictionaries return `V()` — for `Int64` that's `0`, for
    /// `String` that's `""`, etc. Linear in `count`.
    ///
    /// # Examples
    ///
    /// ```
    /// ["a": 1, "b": 2, "c": 3].sumValues();  // 6
    /// [:].sumValues();                        // 0 — V's default
    /// ```
    public func sumValues() -> V {
        var result = V();
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(_, value, _) => {
                    result = result + value;
                },
                _ => {}
            }
        }
        result
    }
}


// ============================================================================
// KEYS VIEW
// ============================================================================

/// Single-pass iterator yielding only the keys of a dictionary.
///
/// Wraps a `DictionaryIterator[K, V]` and discards the value half of
/// each entry. Order matches the underlying entry iteration and is
/// unspecified.
///
/// # Examples
///
/// ```
/// var it = ["a": 1, "b": 2].keys.iter();
/// it.next();  // Some("a")  — order unspecified
/// it.next();  // Some("b")
/// it.next();  // None
/// ```
///
/// # Representation
///
/// Wraps a `DictionaryIterator[K, V]`.
///
/// # Memory Model
///
/// Value type. Aliases dictionary storage; do not retain across
/// mutations.
public struct KeysIterator[K, V]: Iterator where K: Hash {
    /// Element type yielded by `next()` — `K`.
    type Item = K

    /// The underlying entry iterator; only `pair.0` is read.
    private var dictIter: DictionaryIterator[K, V]

    /// @name From Dict
    /// Wraps a `DictionaryIterator` to yield only its keys.
    ///
    /// Prefer `KeysView.iter()` over calling this directly.
    public init(dictIter dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next key, or `None` when the underlying iterator
    /// is exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = ["a": 1].keys.iter();
    /// it.next();  // Some("a")
    /// it.next();  // None
    /// ```
    public mutating func next() -> K? {
        let maybePair = self.dictIter.next();
        if let .Some(pair) = maybePair {
            .Some(pair.0)
        } else {
            .None
        }
    }
}

/// Lazy `Iterable` view over the keys of a dictionary.
///
/// Returned by `Dictionary.keys`. Constructing the view is O(1) — it
/// stores the bucket pointer and capacity. The view is invalidated by
/// any mutation that may reallocate.
///
/// # Examples
///
/// ```
/// let dict = ["a": 1, "b": 2];
/// for k in dict.keys { print(k) }
/// let arr = Array(from: dict.keys);
/// ```
///
/// # Representation
///
/// `(buckets, capacity)` — a pointer into the source dictionary's
/// bucket array plus the total slot count.
///
/// # Memory Model
///
/// Value type that borrows the source dictionary's buffer.
public struct KeysView[K, V]: Iterable where K: Hash {
    /// `Iterable` element type — `K`.
    type Item = K
    /// Concrete iterator type returned by `iter()`.
    type Iter = KeysIterator[K, V]

    /// Pointer into the source dictionary's bucket array.
    private var buckets: Pointer[Bucket[K, V]]
    /// Total slots in the bucket array.
    private var capacity: Int64

    /// @name From Buckets
    /// Internal — constructs a view from a bucket pointer and capacity.
    /// Use `Dictionary.keys` to obtain a view.
    init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
    }

    /// Returns a fresh `KeysIterator[K, V]` over the view.
    ///
    /// Each call returns a new iterator starting at the beginning of
    /// the bucket array.
    public func iter() -> KeysIterator[K, V] {
        KeysIterator(dictIter: DictionaryIterator(buckets: self.buckets, capacity: self.capacity))
    }
}

// ============================================================================
// VALUES VIEW
// ============================================================================

/// Single-pass iterator yielding only the values of a dictionary.
///
/// Wraps a `DictionaryIterator[K, V]` and discards the key half of
/// each entry. Order matches the underlying entry iteration and is
/// unspecified.
///
/// # Examples
///
/// ```
/// var it = ["a": 1, "b": 2].values.iter();
/// it.next();  // Some(1)  — order unspecified
/// it.next();  // Some(2)
/// it.next();  // None
/// ```
///
/// # Representation
///
/// Wraps a `DictionaryIterator[K, V]`.
///
/// # Memory Model
///
/// Value type. Aliases dictionary storage; do not retain across
/// mutations.
public struct ValuesIterator[K, V]: Iterator where K: Hash {
    /// Element type yielded by `next()` — `V`.
    type Item = V

    /// The underlying entry iterator; only `pair.1` is read.
    private var dictIter: DictionaryIterator[K, V]

    /// @name From Dict
    /// Wraps a `DictionaryIterator` to yield only its values.
    ///
    /// Prefer `ValuesView.iter()` over calling this directly.
    public init(dictIter dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next value, or `None` when the underlying iterator
    /// is exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = ["a": 1].values.iter();
    /// it.next();  // Some(1)
    /// it.next();  // None
    /// ```
    public mutating func next() -> V? {
        let maybePair = self.dictIter.next();
        if let .Some(pair) = maybePair {
            .Some(pair.1)
        } else {
            .None
        }
    }
}

/// Lazy `Iterable` view over the values of a dictionary.
///
/// Returned by `Dictionary.values`. Constructing the view is O(1) —
/// it stores the bucket pointer and capacity. The view is invalidated
/// by any mutation that may reallocate.
///
/// # Examples
///
/// ```
/// let dict = ["a": 1, "b": 2];
/// for v in dict.values { print(v) }
/// let sum = dict.values.iter().sum();
/// ```
///
/// # Representation
///
/// `(buckets, capacity)` — a pointer into the source dictionary's
/// bucket array plus the total slot count.
///
/// # Memory Model
///
/// Value type that borrows the source dictionary's buffer.
public struct ValuesView[K, V]: Iterable where K: Hash {
    /// `Iterable` element type — `V`.
    type Item = V
    /// Concrete iterator type returned by `iter()`.
    type Iter = ValuesIterator[K, V]

    /// Pointer into the source dictionary's bucket array.
    private var buckets: Pointer[Bucket[K, V]]
    /// Total slots in the bucket array.
    private var capacity: Int64

    /// @name From Buckets
    /// Internal — constructs a view from a bucket pointer and capacity.
    /// Use `Dictionary.values` to obtain a view.
    init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
    }

    /// Returns a fresh `ValuesIterator[K, V]` over the view.
    ///
    /// Each call returns a new iterator starting at the beginning of
    /// the bucket array.
    public func iter() -> ValuesIterator[K, V] {
        ValuesIterator(dictIter: DictionaryIterator(buckets: self.buckets, capacity: self.capacity))
    }
}


// ============================================================================
// LITERAL CONFORMANCE
// ============================================================================

/// `_ExpressibleByDictionaryLiteral` / `ExpressibleByDictionaryLiteral`
/// / `Defaultable` conformances — what makes the `["a": 1, "b": 2]`
/// literal syntax work for `Dictionary[K, V, H]`.
extend Dictionary[K, V, H]: std.core._ExpressibleByDictionaryLiteral, std.core.ExpressibleByDictionaryLiteral, std.core.Defaultable where K: Hash, H: Hasher, H: Defaultable {
    /// Key type for the literal protocol — equals `K`.
    type Key = K
    /// Value type for the literal protocol — equals `V`.
    type Value = V

    /// @name Literal Bridge
    /// Compiler-emitted bridge for `[k: v, ...]` literals.
    ///
    /// Not called by user code directly — the parser lowers literal
    /// expressions into a `(ptr, count)` pair which this constructor
    /// wraps in a `LiteralSlice` and forwards to
    /// `init(dictionaryLiteral:)`.
    ///
    /// # Safety
    ///
    /// The compiler guarantees `_dictionaryLiteralPointer` points to
    /// exactly `_dictionaryLiteralCount` initialized `(K, V)` pairs.
    public init(_dictionaryLiteralPointer: lang.ptr[(K, V)], _dictionaryLiteralCount: lang.i64) {
        self.init(dictionaryLiteral: std.memory.LiteralSlice(pointer: _dictionaryLiteralPointer, count: _dictionaryLiteralCount))
    }

    /// @name Dictionary Literal
    /// Creates a dictionary by inserting every `(K, V)` pair from a
    /// literal slice in order.
    ///
    /// Last-write-wins on duplicate keys (same as `init(from:)`). An
    /// empty literal yields an empty unallocated dictionary.
    ///
    /// # Examples
    ///
    /// ```
    /// // Triggered by the dictionary-literal syntax:
    /// let dict: [String: Int64] = ["a": 1, "b": 2];
    /// ```
    public init(dictionaryLiteral elements: std.memory.LiteralSlice[(K, V)]) {
        // Create empty dictionary
        self.init();

        // Insert each key-value pair
        var iter = elements.iter();
        while let .Some(pair) = iter.next() {
            let _ = self.insert(pair.0, pair.1);
        }
    }
}

// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Compiler-recognized type alias that lets `[K: V]` desugar to
/// `Dictionary[K, V, DefaultHasher]`.
///
/// Allows annotations like `let m: [String: Int64] = [:]` instead of
/// requiring the user to spell out `Dictionary[String, Int64]`. The
/// hasher is fixed to `DefaultHasher`; for custom hashers, write the
/// `Dictionary[...]` form explicitly.
///
/// # Examples
///
/// ```
/// let counts: [String: Int64] = [:];
/// func tally(of words: [String: Int64]) -> Int64 { ... }
/// ```
@builtin(.DictionaryTypeOperator)
public type DictionaryTypeOperator[K, V] = Dictionary[K, V, DefaultHasher];
