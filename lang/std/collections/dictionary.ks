// Dictionary[K, V] - hash map with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
import std.iter.(Iterator, Iterable)
import std.collections.(DefaultHasher)

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
// DICTIONARY ENTRY
// ============================================================================

/// An entry in the hash table.
public struct DictionaryEntry[K, V] {
    /// The key.
    public var key: K

    /// The value.
    public var value: V

    /// The cached hash value.
    public var hash: UInt64

    /// True if this slot is occupied.
    public var occupied: Bool

    /// True if this slot was deleted (tombstone).
    public var deleted: Bool

    /// Creates an occupied entry.
    public init(key key: K, value value: V, hash hash: UInt64, occupied occupied: Bool, deleted deleted: Bool) {
        self.key = key;
        self.value = value;
        self.hash = hash;
        self.occupied = occupied;
        self.deleted = deleted;
    }

    /// Creates an unoccupied entry with placeholder key/value.
    public init(placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        self.key = placeholderKey;
        self.value = placeholderValue;
        self.hash = UInt64(intLiteral: 0);
        self.occupied = false;
        self.deleted = false;
    }
}

// ============================================================================
// DICTIONARY ITERATOR
// ============================================================================

/// Iterator over dictionary entries.
public struct DictionaryIterator[K, V]: Iterator {
    type Item = DictionaryEntry[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64
    private var index: Int64

    /// Creates a dictionary iterator.
    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
        self.index = Int64(intLiteral: 0);
    }

    /// Returns the next occupied entry, or None if exhausted.
    public mutating func next() -> DictionaryEntry[K, V]? {
        while self.index < self.capacity {
            let entry = self.entries.offset(by: self.index).read();
            self.index = self.index + Int64(intLiteral: 1);
            if entry.occupied {
                return .Some(entry)
            }
        }
        let none: DictionaryEntry[K, V]? = .None;
        none
    }
}

// ============================================================================
// DICTIONARY STORAGE (Internal)
// ============================================================================

/// Internal storage for Dictionary.
struct DictionaryStorage[K, V, H]: Cloneable where K: Hash, H: Hasher, H: Defaultable {
    var entries: Pointer[DictionaryEntry[K, V]]
    var len: Int64
    var cap: Int64
    var placeholderKey: K
    var placeholderValue: V

    init(entries entries: Pointer[DictionaryEntry[K, V]], len len: Int64, cap cap: Int64, placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        self.entries = entries;
        self.len = len;
        self.cap = cap;
        self.placeholderKey = placeholderKey;
        self.placeholderValue = placeholderValue;
    }

    /// Deep clone - allocate new buffer and copy entries.
    func clone() -> DictionaryStorage[K, V, H] {
        if self.cap == Int64(intLiteral: 0) {
            return DictionaryStorage(
                entries: Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0),
                placeholderKey: self.placeholderKey,
                placeholderValue: self.placeholderValue
            )
        }
        let layout = Layout.array[DictionaryEntry[K, V]](self.cap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newEntries = rawPtr.cast[DictionaryEntry[K, V]]();
            // Copy entries
            for i in 0..<self.cap {
                newEntries.offset(by: i).write(self.entries.offset(by: i).read());
            }
            DictionaryStorage(
                entries: newEntries,
                len: self.len,
                cap: self.cap,
                placeholderKey: self.placeholderKey,
                placeholderValue: self.placeholderValue
            )
        } else {
            lang.panic("DictionaryStorage clone allocation failed")
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[DictionaryEntry[K, V]](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.entries.asRaw(), layout)
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
    type Item = DictionaryEntry[K, V]
    type Iter = DictionaryIterator[K, V]

    private var storage: RcBox[DictionaryStorage[K, V, H]]

    // Helper accessors for storage fields
    private func entries() -> Pointer[DictionaryEntry[K, V]] { self.storage.getValue().entries }
    private func len() -> Int64 { self.storage.getValue().len }
    private func cap() -> Int64 { self.storage.getValue().cap }
    private func placeholderKey() -> K { self.storage.getValue().placeholderKey }
    private func placeholderValue() -> V { self.storage.getValue().placeholderValue }

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
    ///
    /// Requires placeholder key/value for internal storage initialization.
    public init(placeholderKey: K, placeholderValue: V) {
        self.storage = RcBox(DictionaryStorage(
            entries: Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0),
            placeholderKey: placeholderKey,
            placeholderValue: placeholderValue
        ));
    }

    /// Creates an empty dictionary with initial capacity.
    ///
    /// Requires placeholder key/value for internal storage initialization.
    public init(capacity capacity: Int64, placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        let actualCap = nextPowerOfTwo(capacity);
        if actualCap > Int64(intLiteral: 0) {
            let layout = Layout.array[DictionaryEntry[K, V]](actualCap);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newEntries = rawPtr.cast[DictionaryEntry[K, V]]();
                // Initialize all entries as unoccupied
                for i in 0..<actualCap {
                    newEntries.offset(by: i).write(DictionaryEntry(
                        placeholderKey: placeholderKey,
                        placeholderValue: placeholderValue
                    ));
                }
                self.storage = RcBox(DictionaryStorage(
                    entries: newEntries,
                    len: Int64(intLiteral: 0),
                    cap: actualCap,
                    placeholderKey: placeholderKey,
                    placeholderValue: placeholderValue
                ))
            } else {
                lang.panic("Dictionary allocation failed")
            }
        } else {
            self.storage = RcBox(DictionaryStorage(
                entries: Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0),
                placeholderKey: placeholderKey,
                placeholderValue: placeholderValue
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

    /// Finds entry by key using hashing and linear probing.
    private func findEntry(key: K) -> Int64? {
        let myCap = self.cap();
        if myCap == Int64(intLiteral: 0) {
            let none: Int64? = .None;
            return none
        }

        let hashValue: UInt64 = self.hashKey(key);
        let capU: UInt64 = UInt64(from: myCap);
        let mod: UInt64 = hashValue.modulo(capU);
        var index: Int64 = Int64(from: mod);
        let myEntries = self.entries();
        var i: Int64 = Int64(intLiteral: 0);

        while i < myCap {
            let entry = myEntries.offset(by: index).read();
            if entry.occupied == false and entry.deleted == false {
                // Found truly empty slot - stop search
                let none: Int64? = .None;
                return none
            }
            if entry.occupied == true and entry.key.equals(key) == true {
                return .Some(index)
            }
            // Linear probing
            index = (index + Int64(intLiteral: 1)) % myCap;
            i = i + Int64(intLiteral: 1)
        }
        let none: Int64? = .None;
        none
    }

    /// Finds first unoccupied slot (either truly empty or deleted).
    private func findEmptySlot(hashValue: UInt64) -> Int64? {
        let myCap = self.cap();
        if myCap == Int64(intLiteral: 0) {
            let none: Int64? = .None;
            return none
        }

        let capU: UInt64 = UInt64(from: myCap);
        let mod: UInt64 = hashValue.modulo(capU);
        var index: Int64 = Int64(from: mod);
        let myEntries = self.entries();
        var i: Int64 = Int64(intLiteral: 0);

        while i < myCap {
            let entry = myEntries.offset(by: index).read();
            if entry.occupied == false {
                return .Some(index)
            }
            // Linear probing
            index = (index + Int64(intLiteral: 1)) % myCap;
            i = i + Int64(intLiteral: 1)
        }
        let none: Int64? = .None;
        none
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

        let oldEntries = s.entries;
        let oldCap = s.cap;

        // Allocate new table
        let layout = Layout.array[DictionaryEntry[K, V]](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newEntries = rawPtr.cast[DictionaryEntry[K, V]]();

            // Initialize new entries
            for i in 0..<newCap {
                newEntries.offset(by: i).write(DictionaryEntry(
                    placeholderKey: s.placeholderKey,
                    placeholderValue: s.placeholderValue
                ));
            }

            // Copy old entries
            var newLen: Int64 = Int64(intLiteral: 0);
            for i in 0..<oldCap {
                let entry = oldEntries.offset(by: i).read();
                if entry.occupied {
                    // Find empty slot in new table using hashing and linear probing
                    let hashValue: UInt64 = entry.hash;
                    let newCapU: UInt64 = UInt64(from: newCap);
                    let mod: UInt64 = hashValue.modulo(newCapU);
                    var slotIndex: Int64 = Int64(from: mod);
                    var foundSlot: Bool = false;
                    while foundSlot == false {
                        let slotEntry = newEntries.offset(by: slotIndex).read();
                        if slotEntry.occupied == false {
                            newEntries.offset(by: slotIndex).write(entry);
                            newLen = newLen + Int64(intLiteral: 1);
                            foundSlot = true
                        } else {
                            slotIndex = (slotIndex + Int64(intLiteral: 1)) % newCap
                        }
                    }
                }
            }

            // Free old table
            if oldCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[DictionaryEntry[K, V]](oldCap);
                allocator.deallocate(oldEntries.asRaw(), oldLayout)
            }

            self.storage.setValue(DictionaryStorage(
                entries: newEntries,
                len: newLen,
                cap: newCap,
                placeholderKey: s.placeholderKey,
                placeholderValue: s.placeholderValue
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
            let entry = self.entries().offset(by: index).read();
            .Some(entry.value)
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
            let myEntries = self.entries();
            let oldEntry = myEntries.offset(by: index).read();
            myEntries.offset(by: index).write(DictionaryEntry(
                key: key,
                value: value,
                hash: hashValue,
                occupied: true,
                deleted: false
            ));
            return .Some(oldEntry.value)
        }

        // Need to insert - ensure capacity first
        self.ensureCapacity();
        self.makeUnique();

        // Find empty slot
        let maybeSlot = self.findEmptySlot(hashValue);
        if let .Some(slotIndex) = maybeSlot {
            var s = self.storage.getValue();
            s.entries.offset(by: slotIndex).write(DictionaryEntry(
                key: key,
                value: value,
                hash: hashValue,
                occupied: true,
                deleted: false
            ));
            s.len = s.len + Int64(intLiteral: 1);
            self.storage.setValue(s)
        } else {
            lang.panic("Dictionary insert failed - no empty slot")
        }

        let none: V? = .None;
        none
    }

    /// Removes the key and returns its value, or None if not found.
    public mutating func remove(key: K) -> V? {
        let maybeIndex = self.findEntry(key);

        if let .Some(index) = maybeIndex {
            self.makeUnique();
            var s = self.storage.getValue();
            let entry = s.entries.offset(by: index).read();
            let removedValue = entry.value;

            // Mark as unoccupied and deleted (tombstone)
            s.entries.offset(by: index).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: entry.hash,
                occupied: false,
                deleted: true
            ));
            s.len = s.len - Int64(intLiteral: 1);
            self.storage.setValue(s);

            return .Some(removedValue)
        }

        let none: V? = .None;
        none
    }

    /// Removes all entries from the dictionary.
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        for i in 0..<s.cap {
            let entry = s.entries.offset(by: i).read();
            // Mark unoccupied and not deleted
            s.entries.offset(by: i).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: UInt64(intLiteral: 0),
                occupied: false,
                deleted: false
            ));
        }
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the entries.
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(entries: self.entries(), capacity: self.cap())
    }

    /// Returns the internal entries pointer for views.
    public func getEntries() -> Pointer[DictionaryEntry[K, V]] { self.entries() }

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
        var equal: Bool = true;
        let selfCap = self.getCapacity();
        let selfEntries = self.getEntries();
        for i in 0..<selfCap {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                let otherValue = other.getValue(entry.key);
                if let .Some(value) = otherValue {
                    if entry.value.equals(value) == false {
                        equal = false
                    }
                } else {
                    equal = false
                }
            }
            if equal == false {
                return false
            }
        }
        equal
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
    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next key, or None if exhausted.
    public mutating func next() -> K? {
        let maybeEntry = self.dictIter.next();
        if let .Some(entry) = maybeEntry {
            .Some(entry.key)
        } else {
            .None
        }
    }
}

/// A view over dictionary keys.
public struct KeysView[K, V]: Iterable where K: Hash {
    type Item = K
    type Iter = KeysIterator[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64

    /// Creates a keys view.
    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
    }

    /// Returns an iterator over the keys.
    public func iter() -> KeysIterator[K, V] {
        KeysIterator(DictionaryIterator(entries: self.entries, capacity: self.capacity))
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
    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    /// Returns the next value, or None if exhausted.
    public mutating func next() -> V? {
        let maybeEntry = self.dictIter.next();
        if let .Some(entry) = maybeEntry {
            .Some(entry.value)
        } else {
            .None
        }
    }
}

/// A view over dictionary values.
public struct ValuesView[K, V]: Iterable where K: Hash {
    type Item = V
    type Iter = ValuesIterator[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64

    /// Creates a values view.
    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
    }

    /// Returns an iterator over the values.
    public func iter() -> ValuesIterator[K, V] {
        ValuesIterator(DictionaryIterator(entries: self.entries, capacity: self.capacity))
    }
}

// ============================================================================
// VIEW EXTENSIONS
// ============================================================================

/// Extension to add keys() and values() methods.
extend Dictionary[K, V, H] where K: Hash, H: Hasher, H: Defaultable {
    /// Returns a view over the keys.
    public func keys() -> KeysView[K, V] {
        KeysView(entries: self.getEntries(), capacity: self.getCapacity())
    }

    /// Returns a view over the values.
    public func values() -> ValuesView[K, V] {
        ValuesView(entries: self.getEntries(), capacity: self.getCapacity())
    }
}

// ============================================================================
// LITERAL CONFORMANCE
// ============================================================================

/// ExpressibleByDictionaryLiteral conformance.
extend Dictionary[K, V, H]: std.core._ExpressibleByDictionaryLiteral, std.core.ExpressibleByDictionaryLiteral where K: Hash, K: Defaultable, V: Defaultable, H: Hasher, H: Defaultable {
    type Key = K
    type Value = V

    /// Internal initializer called by compiler for dictionary literals.
    public init(_dictionaryLiteralPointer: lang.ptr[(K, V)], _dictionaryLiteralCount: lang.i64) {
        self.init(dictionaryLiteral: std.memory.LiteralSlice(pointer: _dictionaryLiteralPointer, count: _dictionaryLiteralCount))
    }

    /// Creates a dictionary from a dictionary literal.
    public init(dictionaryLiteral elements: std.memory.LiteralSlice[(K, V)]) {
        // Create empty dictionary with default placeholders
        let placeholderKey: K = K();
        let placeholderValue: V = V();
        self.init(placeholderKey: placeholderKey, placeholderValue: placeholderValue);

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
