// Dictionary[K, V] - hash map with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
import std.iter.(Iterator, Iterable)
import std.collections.(DefaultHasher)

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
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of key-value pairs.
    public func count() -> Int64 { self.len() }

    /// The allocated capacity.
    public func getCapacity() -> Int64 { self.cap() }

    /// True if the dictionary is empty.
    public func isEmpty() -> Bool { self.len() == Int64(intLiteral: 0) }

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

    // ========================================================================
    // VALUE ACCESS
    // ========================================================================

    /// Returns the value for the given key, or None if not found.
    public func getValue(key: K) -> V? {
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
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        for i in 0..<s.cap {
            s.buckets.offset(by: i).write(.Empty);
        }
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
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
    public func equals(other: Dictionary[K, V, H]) -> Bool {
        let selfCount = self.count();
        let otherCount = other.count();
        if selfCount != otherCount {
            return false
        }

        // Check all entries in self exist in other with same value
        let selfCap = self.getCapacity();
        let selfBuckets = self.getBuckets();
        for i in 0..<selfCap {
            let bucket = selfBuckets.offset(by: i).read();
            match bucket {
                .Occupied(key, value, _) => {
                    let otherValue = other.getValue(key);
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
// VIEW EXTENSIONS
// ============================================================================

/// Extension to add keys() and values() methods.
extend Dictionary[K, V, H] where K: Hash, H: Hasher, H: Defaultable {
    /// Returns a view over the keys.
    public func keys() -> KeysView[K, V] {
        KeysView(buckets: self.getBuckets(), capacity: self.getCapacity())
    }

    /// Returns a view over the values.
    public func values() -> ValuesView[K, V] {
        ValuesView(buckets: self.getBuckets(), capacity: self.getCapacity())
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
