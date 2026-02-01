// Dictionary[K, V] - hash map with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable, Formattable, Addable)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
import std.iter.(Iterator, Iterable)
import std.collections.(DefaultHasher, Array)

// ============================================================================
// BUCKET ENUM
// ============================================================================

/// Represents a slot in the hash table.
public enum Bucket[K, V] {
    /// Empty slot - never been used.
    case Empty

    /// Deleted slot - was occupied, now removed (tombstone).
    case Deleted

    /// Occupied slot with key, value, and cached hash.
    case Occupied(K, V, UInt64)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Computes the next power of two, minimum 8.
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

/// Iterator over dictionary key-value pairs.
public struct DictionaryIterator[K, V]: Iterator {
    type Item = (K, V)

    private var buckets: Pointer[Bucket[K, V]]
    private var capacity: Int64
    private var index: Int64

    /// Creates a dictionary iterator.
    public init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next key-value pair, or None if exhausted.
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

/// Internal storage for Dictionary.
struct DictionaryStorage[K, V, H]: Cloneable where K: Hash, H: Hasher, H: Defaultable {
    var buckets: Pointer[Bucket[K, V]]
    var len: Int64
    var cap: Int64

    init(buckets buckets: Pointer[Bucket[K, V]], len len: Int64, cap cap: Int64) {
        self.buckets = buckets;
        self.len = len;
        self.cap = cap;
    }

    /// Deep clone - allocate new buffer and copy buckets.
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

/// A hash map with copy-on-write semantics.
///
/// Uses open addressing with linear probing. Keys must implement Hash.
public struct Dictionary[K, V, H = DefaultHasher]: Iterable, Cloneable where K: Hash, H: Hasher, H: Defaultable {
    type Item = (K, V)
    type Iter = DictionaryIterator[K, V]

    private var storage: RcBox[DictionaryStorage[K, V, H]]

    // Helper accessors for storage fields
    private func buckets() -> Pointer[Bucket[K, V]] { self.storage.getValue().buckets }
    private func len() -> Int64 { self.storage.getValue().len }
    private func cap() -> Int64 { self.storage.getValue().cap }

    // Ensure unique storage for mutation (COW)
    private mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = self.storage.deepClone()
        }
    }

    /// Private init for internal use (from storage).
    private init(storage storage: RcBox[DictionaryStorage[K, V, H]]) {
        self.storage = storage;
    }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an empty dictionary.
    public init() {
        self.storage = RcBox(DictionaryStorage(
            buckets: Pointer(raw: lang.ptr_null[Bucket[K, V]]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    /// Creates an empty dictionary with initial capacity.
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

    /// Creates a dictionary from an iterable of key-value tuples.
    ///
    /// If duplicate keys exist, later values overwrite earlier ones.
    ///
    /// Example:
    ///     let pairs = [("a", 1), ("b", 2)]
    ///     let dict = Dictionary(from: pairs)  // ["a": 1, "b": 2]
    public init[I](from pairs: I) where I: Iterable, I.Item = (K, V) {
        self.init();
        var iter = pairs.iter();
        while let .Some(pair) = iter.next() {
            let _ = self.insert( pair.0, pair.1);
        }
    }

    /// Creates a dictionary by grouping elements by a key function.
    ///
    /// Each key maps to an array of all elements that produced that key.
    /// Note: This initializer requires V = Array[E].
    ///
    /// Example:
    ///     let words = ["apple", "apricot", "banana", "blueberry"]
    ///     let grouped = Dictionary(grouping: words, by: { (w) in w.chars.first().unwrap() })
    ///     // ["a": ["apple", "apricot"], "b": ["banana", "blueberry"]]
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

    /// Creates a dictionary from unique key-value pairs.
    ///
    /// Panics if duplicate keys are encountered. Use `init(from:)` if
    /// duplicates should be allowed (with later values winning).
    ///
    /// Example:
    ///     let dict = Dictionary(uniqueKeysWithValues: [("a", 1), ("b", 2)])
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

    /// Hashes a key using the generic hasher H.
    private func hashKey(key: K) -> UInt64 {
        var hasher = H();
        key.hash(into: hasher);
        hasher.finish()
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Returns the number of key-value pairs in the dictionary.
    ///
    /// Example:
    ///     ["a": 1, "b": 2].count  // 2
    ///     [:].count               // 0
    public var count: Int64 { get { self.len() } }

    /// Returns the current capacity (pairs storable without reallocating).
    ///
    /// Capacity is always >= count. When count exceeds capacity, the dictionary
    /// reallocates with increased capacity.
    ///
    /// Example:
    ///     var dict = Dictionary[String, Int64](capacity: 100)
    ///     dict.capacity  // >= 100
    public var capacity: Int64 { get { self.cap() } }

    /// Returns true if the dictionary contains no key-value pairs.
    ///
    /// Example:
    ///     [:].isEmpty           // true
    ///     ["a": 1].isEmpty      // false
    public var isEmpty: Bool { get { self.len() == Int64(intLiteral: 0) } }

    /// Returns a view of the dictionary's keys.
    ///
    /// The view is lazy and iterates over keys without copying.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2, "c": 3]
    ///     for key in dict.keys {
    ///         print(key)
    ///     }
    ///     let keyArray = Array(from: dict.keys)  // ["a", "b", "c"]
    public var keys: KeysView[K, V] { get { KeysView(buckets: self.getBuckets(), capacity: self.cap()) } }

    /// Returns a view of the dictionary's values.
    ///
    /// The view is lazy and iterates over values without copying.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2, "c": 3]
    ///     for value in dict.values {
    ///         print(value)
    ///     }
    ///     let sum = dict.values.iter().sum()  // 6
    public var values: ValuesView[K, V] { get { ValuesView(buckets: self.getBuckets(), capacity: self.cap()) } }

    // ========================================================================
    // SUBSCRIPTS
    // ========================================================================

    /// Accesses the value for the given key.
    ///
    /// Returns None if the key doesn't exist. Setting to None removes the key.
    ///
    /// Example:
    ///     var dict = ["a": 1, "b": 2]
    ///     dict("a")           // Some(1)
    ///     dict("z")           // None
    ///     dict("c") = 3       // inserts "c": 3
    ///     dict("a") = None    // removes "a"
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

    /// Accesses the value for the given key, returning a default if missing.
    ///
    /// The default is NOT inserted into the dictionary.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2]
    ///     dict("a", default: 0)  // 1
    ///     dict("z", default: 0)  // 0
    ///     dict("z")              // still None
    public subscript(key: K, default defaultValue: V) -> V {
        get {
            let maybeValue = self(key);
            match maybeValue {
                .Some(v) => v,
                .None => defaultValue
            }
        }
    }

    /// Accesses the value for the given key, inserting a default if missing.
    ///
    /// If the key doesn't exist, the default is inserted and returned.
    /// Useful for accumulating values.
    ///
    /// Example:
    ///     var counts: [String: Int64] = [:]
    ///     counts("apple", inserting: 0) += 1  // inserts 0, then increments
    ///     counts("apple", inserting: 0) += 1  // just increments
    ///     counts("apple")  // Some(2)
    public subscript(key: K, inserting defaultValue: V) -> V {
        get {
            let maybeValue = self(key);
            match maybeValue {
                .Some(v) => v,
                .None => {
                    let _ = self.insert( key, defaultValue);
                    defaultValue
                }
            }
        }
        set {
            let _ = self.insert( key, newValue);
        }
    }

    /// Accesses the value for the given key, panicking if missing.
    ///
    /// Use when you're certain the key exists.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2]
    ///     dict(unwrap: "a")  // 1
    ///     dict(unwrap: "z")  // PANIC: key not found
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

    /// Finds bucket index by key using hashing and linear probing.
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

    /// Finds first unoccupied slot (either empty or deleted).
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

    /// Ensures capacity for more entries (resizes at 75% load).
    private mutating func ensureCapacity() {
        let myCap = self.cap();
        let myLen = self.len();
        let threshold = myCap * Int64(intLiteral: 3) / Int64(intLiteral: 4);
        if myLen >= threshold or myCap == Int64(intLiteral: 0) {
            self.resize()
        }
    }

    /// Resizes the hash table to double capacity.
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

    /// Resizes the hash table to specific capacity.
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

    /// Returns true if the dictionary contains the given key.
    public func contains(key: K) -> Bool {
        self.findEntry(key).isSome()
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Inserts or updates a value for the given key.
    ///
    /// Returns the old value if the key already existed.
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

    /// Removes the key and returns its value, or None if not found.
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

    /// Removes all entries from the dictionary.
    ///
    /// Capacity may be retained for reuse.
    ///
    /// Example:
    ///     var dict = ["a": 1, "b": 2]
    ///     dict.clear()  // dict is now [:]
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        for i in 0..<s.cap {
            s.buckets.offset(by: i).write(.Empty);
        }
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    /// Updates the value for an existing key using a transform function.
    ///
    /// Returns true if the key existed and was updated, false otherwise.
    ///
    /// Example:
    ///     var dict = ["a": 1, "b": 2]
    ///     dict.update(key: "a", with: { (v) in v * 10 })  // true, dict["a"] = 10
    ///     dict.update(key: "z", with: { (v) in v * 10 })  // false, no change
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

    /// Updates or inserts: applies transform if key exists, otherwise inserts default.
    ///
    /// Example:
    ///     var counts: [String: Int64] = [:]
    ///     counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 })
    ///     counts.upsert(key: "apple", default: 0, with: { (n) in n + 1 })
    ///     counts("apple")  // Some(2)
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

    /// Merges another dictionary into this one.
    ///
    /// For duplicate keys, the combine function determines the resulting value.
    ///
    /// Example:
    ///     var a = ["x": 1, "y": 2]
    ///     let b = ["y": 20, "z": 30]
    ///     a.merge( b, uniquingKeysWith: { (old, new) in old + new })
    ///     // a is now ["x": 1, "y": 22, "z": 30]
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

    /// Merges key-value pairs from an iterable into this dictionary.
    ///
    /// Example:
    ///     var dict = ["a": 1]
    ///     dict.mergeFrom([("b", 2), ("c", 3)], uniquingKeysWith: { (_, new) in new })
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

    /// Retains only entries that satisfy the predicate.
    ///
    /// Example:
    ///     var dict = ["a": 1, "b": 2, "c": 3]
    ///     dict.retain(where: { (k, v) in v > 1 })  // ["b": 2, "c": 3]
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

    /// Removes all entries that satisfy the predicate.
    ///
    /// The inverse of `retain(where:)`.
    ///
    /// Example:
    ///     var dict = ["a": 1, "b": 2, "c": 3]
    ///     dict.removeAll(where: { (k, v) in v < 2 })  // ["b": 2, "c": 3]
    public mutating func removeAll(matching predicate: (K, V) -> Bool) {
        self.retain(matching: { (k, v) in predicate(k, v) == false })
    }

    /// Reserves capacity for at least minimumCapacity entries.
    ///
    /// Does nothing if current capacity is already sufficient.
    ///
    /// Example:
    ///     var dict = Dictionary[String, Int64]()
    ///     dict.reserveCapacity(minimumCapacity: 1000)
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        if minimumCapacity <= self.capacity {
            return
        }
        // Calculate target capacity (accounting for load factor)
        let targetCap = nextPowerOfTwo(minimumCapacity * Int64(intLiteral: 4) / Int64(intLiteral: 3));
        self.resizeToCapacity(targetCap)
    }

    /// Reduces capacity to match the current count.
    ///
    /// Frees excess memory. Useful after removing many entries.
    ///
    /// Example:
    ///     var dict = Dictionary[String, Int64](capacity: 1000)
    ///     dict("a") = 1
    ///     dict.shrinkToFit()  // capacity reduced
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

    /// Returns an iterator over key-value pairs.
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(buckets: self.buckets(), capacity: self.cap())
    }

    /// Returns the internal buckets pointer for views.
    public func getBuckets() -> Pointer[Bucket[K, V]] { self.buckets() }

    // ========================================================================
    // SEARCHING AND PREDICATES
    // ========================================================================

    /// Returns true if any entry satisfies the predicate.
    ///
    /// Returns false for an empty dictionary.
    /// Short-circuits on first matching entry.
    ///
    /// Example:
    ///     ["a": 1, "b": 5].contains(where: { (k, v) in v > 3 })  // true
    ///     ["a": 1, "b": 2].contains(where: { (k, v) in v > 3 })  // false
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

    /// Returns the first entry matching the predicate, or None.
    ///
    /// Note: Since dictionary order is unspecified, "first" is arbitrary.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 5, "c": 3]
    ///     dict.first(where: { (k, v) in v > 2 })  // Some entry with v > 2
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

    /// Returns true if all entries satisfy the predicate.
    ///
    /// Returns true for an empty dictionary (vacuous truth).
    ///
    /// Example:
    ///     ["a": 2, "b": 4].all(satisfy: { (k, v) in v % 2 == 0 })  // true
    ///     ["a": 1, "b": 2].all(satisfy: { (k, v) in v % 2 == 0 })  // false
    ///     [:].all(satisfy: { (k, v) in false })                    // true
    public func all(satisfy predicate: (K, V) -> Bool) -> Bool {
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

    /// Returns true if any entry satisfies the predicate.
    ///
    /// Alias for `contains(where:)`.
    ///
    /// Example:
    ///     ["a": 1, "b": 5].any(satisfy: { (k, v) in v > 3 })  // true
    public func any(satisfy predicate: (K, V) -> Bool) -> Bool {
        self.contains(matching: predicate)
    }

    /// Returns the count of entries satisfying the predicate.
    ///
    /// Example:
    ///     ["a": 1, "b": 2, "c": 3].countWhere({ (k, v) in v > 1 })  // 2
    public func countWhere(predicate: (K, V) -> Bool) -> Int64 {
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

    /// Returns a new dictionary with transformed values.
    ///
    /// Keys are preserved; only values are transformed.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2]
    ///     let doubled = dict.mapValues { it * 2 }  // ["a": 2, "b": 4]
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

    /// Returns a new dictionary with transformed values, removing None results.
    ///
    /// Example:
    ///     let dict = ["a": "1", "b": "two", "c": "3"]
    ///     let parsed = dict.compactMapValues { Int64.parse(it) }
    ///     // ["a": 1, "c": 3] - "two" couldn't be parsed
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

    /// Returns a new dictionary with only entries satisfying the predicate.
    ///
    /// Example:
    ///     let dict = ["a": 1, "b": 2, "c": 3]
    ///     let big = dict.filter(where: { (k, v) in v > 1 })  // ["b": 2, "c": 3]
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

    /// Returns a new dictionary merging this one with another.
    ///
    /// This dictionary is unchanged.
    ///
    /// Example:
    ///     let a = ["x": 1, "y": 2]
    ///     let b = ["y": 20, "z": 30]
    ///     let merged = a.merging(other: b, uniquingKeysWith: { (_, new) in new })
    ///     // merged is ["x": 1, "y": 20, "z": 30]
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

    /// Creates a shallow clone (COW - copy deferred until mutation).
    public func clone() -> Dictionary[K, V, H] {
        Dictionary(storage: self.storage.clone())
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// Equatable extension when K and V are Equatable.
extend Dictionary[K, V, H]: Equatable where K: Hash, V: Equatable, H: Hasher, H: Defaultable {
    /// Compares two dictionaries for equality.
    ///
    /// Two dictionaries are equal if they have the same keys and each key
    /// maps to an equal value.
    ///
    /// Example:
    ///     ["a": 1, "b": 2].equals(other: ["b": 2, "a": 1])  // true
    ///     ["a": 1].equals(other: ["a": 2])                   // false
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

/// Extension for dictionaries where value is Equatable.
extend Dictionary[K, V, H] where K: Hash, V: Equatable, H: Hasher, H: Defaultable {

    /// Returns true if the dictionary contains the given value.
    ///
    /// Note: O(n) - must scan all values.
    ///
    /// Example:
    ///     ["a": 1, "b": 2].containsValue(2)  // true
    ///     ["a": 1, "b": 2].containsValue(5)  // false
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

    /// Returns the first key mapping to the given value, or None.
    ///
    /// Note: O(n) and "first" is arbitrary since order is unspecified.
    ///
    /// Example:
    ///     ["a": 1, "b": 2].firstKey(forValue: 2)  // Some("b")
    ///     ["a": 1, "b": 2].firstKey(forValue: 5)  // None
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

    /// Returns all keys mapping to the given value.
    ///
    /// Example:
    ///     ["a": 1, "b": 2, "c": 1].allKeys(forValue: 1)  // ["a", "c"]
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

/// Formattable conformance for dictionaries with formattable keys and values.
///
/// Dictionaries format as "{key1: value1, key2: value2}".
/// Debug mode shows type: "Dictionary[String, Int64]{a: 1, b: 2}".
///
/// Example:
///     "\{["a": 1, "b": 2]}"     // "{a: 1, b: 2}"
///     "\{["a": 1, "b": 2]:?}"   // "Dictionary[String, Int64]{a: 1, b: 2}"
extend Dictionary[K, V, H]: Formattable where K: Hash, K: Formattable, V: Formattable, H: Hasher, H: Defaultable {
    /// Formats this dictionary as a string.
    public func format() -> String {
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
                    result = result + key.format() + ": " + value.format();
                },
                _ => {}
            }
        }
        result + "}"
    }
}

/// Deep clone when both K and V are Cloneable.
extend Dictionary[K, V, H] where K: Hash, K: Cloneable, V: Cloneable, H: Hasher, H: Defaultable {

    /// Creates a deep clone of the dictionary.
    ///
    /// Unlike `clone()` which shares storage via COW, this immediately
    /// copies all keys and values.
    ///
    /// Example:
    ///     let a = ["x": [1, 2, 3]]
    ///     let b = a.deepClone()  // fully independent copy
    public func deepClone() -> Dictionary[K, V, H] {
        var result = Dictionary[K, V, H](capacity: self.capacity);
        let myCap = self.capacity;
        let myBuckets = self.getBuckets();

        for i in 0..<myCap {
            let bucket = myBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    let _ = result.insert( key.clone(), value.clone());
                },
                _ => {}
            }
        }
        result
    }
}

/// Extension for dictionaries with numeric values.
extend Dictionary[K, V, H] where K: Hash, V: Addable, V: Defaultable, H: Hasher, H: Defaultable {

    /// Returns the sum of all values.
    ///
    /// Example:
    ///     ["a": 1, "b": 2, "c": 3].sumValues()  // 6
    ///     [:].sumValues()                        // 0 (default)
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

/// Iterator over dictionary keys.
public struct KeysIterator[K, V]: Iterator where K: Hash {
    type Item = K

    private var dictIter: DictionaryIterator[K, V]

    /// Creates a keys iterator.
    public init(dictIter dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next key, or None if exhausted.
    public mutating func next() -> K? {
        let maybePair = self.dictIter.next();
        if let .Some(pair) = maybePair {
            .Some(pair.0)
        } else {
            .None
        }
    }
}

/// A view over dictionary keys.
public struct KeysView[K, V]: Iterable where K: Hash {
    type Item = K
    type Iter = KeysIterator[K, V]

    private var buckets: Pointer[Bucket[K, V]]
    private var capacity: Int64

    /// Creates a keys view.
    public init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
    }

    /// Returns an iterator over the keys.
    public func iter() -> KeysIterator[K, V] {
        KeysIterator(dictIter: DictionaryIterator(buckets: self.buckets, capacity: self.capacity))
    }
}

// ============================================================================
// VALUES VIEW
// ============================================================================

/// Iterator over dictionary values.
public struct ValuesIterator[K, V]: Iterator where K: Hash {
    type Item = V

    private var dictIter: DictionaryIterator[K, V]

    /// Creates a values iterator.
    public init(dictIter dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next value, or None if exhausted.
    public mutating func next() -> V? {
        let maybePair = self.dictIter.next();
        if let .Some(pair) = maybePair {
            .Some(pair.1)
        } else {
            .None
        }
    }
}

/// A view over dictionary values.
public struct ValuesView[K, V]: Iterable where K: Hash {
    type Item = V
    type Iter = ValuesIterator[K, V]

    private var buckets: Pointer[Bucket[K, V]]
    private var capacity: Int64

    /// Creates a values view.
    public init(buckets buckets: Pointer[Bucket[K, V]], capacity capacity: Int64) {
        self.buckets = buckets;
        self.capacity = capacity;
    }

    /// Returns an iterator over the values.
    public func iter() -> ValuesIterator[K, V] {
        ValuesIterator(dictIter: DictionaryIterator(buckets: self.buckets, capacity: self.capacity))
    }
}


// ============================================================================
// LITERAL CONFORMANCE
// ============================================================================

/// ExpressibleByDictionaryLiteral conformance.
extend Dictionary[K, V, H]: std.core._ExpressibleByDictionaryLiteral, std.core.ExpressibleByDictionaryLiteral, std.core.Defaultable where K: Hash, H: Hasher, H: Defaultable {
    type Key = K
    type Value = V

    /// Internal initializer called by compiler for dictionary literals.
    public init(_dictionaryLiteralPointer: lang.ptr[(K, V)], _dictionaryLiteralCount: lang.i64) {
        self.init(dictionaryLiteral: std.memory.LiteralSlice(pointer: _dictionaryLiteralPointer, count: _dictionaryLiteralCount))
    }

    /// Creates a dictionary from a dictionary literal.
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

/// Type operator alias: [K: V] desugars to Dictionary[K, V].
@builtin(.DictionaryTypeOperator)
public type DictionaryTypeOperator[K, V] = Dictionary[K, V, DefaultHasher];
