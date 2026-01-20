// Dictionary[K, V] - hash map

module std.collections

import std.core.(Bool, Equatable, Cloneable)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator)
import std.iter.(Iterator, Iterable)

// Compute next power of two, minimum 8
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

// Entry in the hash table
public struct DictionaryEntry[K, V] {
    public var key: K
    public var value: V
    public var hash: UInt64
    public var occupied: Bool

    public init(key key: K, value value: V, hash hash: UInt64, occupied occupied: Bool) {
        self.key = key;
        self.value = value;
        self.hash = hash;
        self.occupied = occupied;
    }

    // Create an unoccupied entry with placeholder key/value
    public init(placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        self.key = placeholderKey;
        self.value = placeholderValue;
        self.hash = UInt64(intLiteral: 0);
        self.occupied = false;
    }
}

// DictionaryIterator must be defined before Dictionary for Iterable conformance
public struct DictionaryIterator[K, V]: Iterator {
    type Item = DictionaryEntry[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64
    private var index: Int64

    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
        self.index = Int64(intLiteral: 0);
    }

    public mutating func next() -> Optional[DictionaryEntry[K, V]] {
        while self.index < self.capacity {
            let entry = self.entries.offset(by: self.index).read();
            self.index = self.index + Int64(intLiteral: 1);
            if entry.occupied {
                return .Some(entry)
            }
        }
        let none: Optional[DictionaryEntry[K, V]] = .None;
        none
    }
}

// Dictionary[K, V] - simple hash map using open addressing with linear probing
// Note: Keys must be Equatable. Proper hashing requires Hashable which isn't fully implemented.
// This implementation uses a simple hash based on the raw bytes of the key.
public struct Dictionary[K, V]: Iterable where K: Equatable {
    type Item = DictionaryEntry[K, V]
    type Iter = DictionaryIterator[K, V]

    // Made internal (not private) so extensions can access
    var entries: Pointer[DictionaryEntry[K, V]]
    var len: Int64
    var cap: Int64
    var placeholderKey: K
    var placeholderValue: V

    // Create empty dictionary - requires placeholder key/value for future resizing
    public init(placeholderKey: K, placeholderValue: V) {
        self.entries = Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]());
        self.len = Int64(intLiteral: 0);
        self.cap = Int64(intLiteral: 0);
        self.placeholderKey = placeholderKey;
        self.placeholderValue = placeholderValue;
    }

    // Create with initial capacity - requires a placeholder key/value for initialization
    public init(capacity capacity: Int64, placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        let actualCap = nextPowerOfTwo(capacity);
        self.placeholderKey = placeholderKey;
        self.placeholderValue = placeholderValue;
        if actualCap > Int64(intLiteral: 0) {
            let layout = Layout.array[DictionaryEntry[K, V]](actualCap);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                self.entries = result.unwrap().cast[DictionaryEntry[K, V]]();
                self.len = Int64(intLiteral: 0);
                self.cap = actualCap;
                // Initialize all entries as unoccupied
                var i: Int64 = Int64(intLiteral: 0);
                while i < actualCap {
                    self.entries.offset(by: i).write(DictionaryEntry(
                        placeholderKey: placeholderKey,
                        placeholderValue: placeholderValue
                    ));
                    i = i + Int64(intLiteral: 1)
                }
            } else {
                lang.panic("Dictionary allocation failed")
            }
        } else {
            self.entries = Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]());
            self.len = Int64(intLiteral: 0);
            self.cap = Int64(intLiteral: 0)
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[DictionaryEntry[K, V]](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.entries.asRaw(), layout)
        }
    }

    // Simple hash function - XOR shift based on the UInt64 representation
    // This works for types whose first 8 bytes are meaningful (Int64, UInt64, pointers, etc.)
    // For proper hashing, types should implement Hashable protocol
    private func hashKey(key: K) -> UInt64 {
        // Read the key's memory as raw bytes and compute a simple hash
        // This is a simplified approach - proper implementation needs Hashable
        let keyPtr = self.entries;  // Dummy - we just need something to satisfy the compiler
        // Use a constant hash for now (all keys hash to same bucket - O(n) but correct)
        // TODO: Implement proper hashing when Hashable protocol is complete
        UInt64(intLiteral: 0)
    }

    // Properties
    public func count() -> Int64 { self.len }
    public func getCapacity() -> Int64 { self.cap }
    public func isEmpty() -> Bool { self.len == Int64(intLiteral: 0) }

    // Find entry by key using linear search (since hash is constant)
    private func findEntry(key: K) -> Optional[Int64] {
        if self.cap == Int64(intLiteral: 0) {
            let none: Optional[Int64] = .None;
            return none
        }

        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < self.cap and done == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied == true and entry.key.equals(key) == true {
                result = .Some(i);
                done = true
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Find first unoccupied slot
    private func findEmptySlot() -> Optional[Int64] {
        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < self.cap and done == false {
            let entry = self.entries.offset(by: i).read();
            if entry.occupied == false {
                result = .Some(i);
                done = true
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }

    // Ensure we have capacity for more entries (resize at 75% load)
    private mutating func ensureCapacity() {
        let threshold = self.cap * Int64(intLiteral: 3) / Int64(intLiteral: 4);
        if self.len >= threshold or self.cap == Int64(intLiteral: 0) {
            self.resize(self.placeholderKey, self.placeholderValue)
        }
    }

    // Resize the hash table
    private mutating func resize(placeholderKey: K, placeholderValue: V) {
        let newCap: Int64 = if self.cap == Int64(intLiteral: 0) {
            Int64(intLiteral: 8)
        } else {
            self.cap * Int64(intLiteral: 2)
        };

        let oldEntries = self.entries;
        let oldCap = self.cap;

        // Allocate new table
        let layout = Layout.array[DictionaryEntry[K, V]](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if result.isSome() {
            self.entries = result.unwrap().cast[DictionaryEntry[K, V]]();
            self.cap = newCap;
            self.len = Int64(intLiteral: 0);

            // Initialize new entries
            var i: Int64 = Int64(intLiteral: 0);
            while i < newCap {
                self.entries.offset(by: i).write(DictionaryEntry(
                    placeholderKey: placeholderKey,
                    placeholderValue: placeholderValue
                ));
                i = i + Int64(intLiteral: 1)
            }

            // Copy old entries
            i = Int64(intLiteral: 0);
            while i < oldCap {
                let entry = oldEntries.offset(by: i).read();
                if entry.occupied {
                    // Find empty slot and insert
                    let maybeSlot = self.findEmptySlot();
                    if maybeSlot.isSome() {
                        let slotIndex = maybeSlot.unwrap();
                        self.entries.offset(by: slotIndex).write(entry);
                        self.len = self.len + Int64(intLiteral: 1)
                    } else {
                        lang.panic("Dictionary resize failed - no empty slot found")
                    }
                }
                i = i + Int64(intLiteral: 1)
            }

            // Free old table
            if oldCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[DictionaryEntry[K, V]](oldCap);
                allocator.deallocate(oldEntries.asRaw(), oldLayout)
            }
        } else {
            lang.panic("Dictionary resize failed")
        }
    }

    // Get value for key
    public func getValue(key: K) -> Optional[V] {
        let maybeIndex = self.findEntry(key);
        if maybeIndex.isSome() {
            let entry = self.entries.offset(by: maybeIndex.unwrap()).read();
            .Some(entry.value)
        } else {
            .None
        }
    }

    // Check if key exists
    public func contains(key: K) -> Bool {
        self.findEntry(key).isSome()
    }

    // Insert or update value for key, returns old value if any
    public mutating func insert(key: K, value: V) -> Optional[V] {
        // Check if key already exists
        let maybeIndex = self.findEntry(key);
        if maybeIndex.isSome() == true {
            let index = maybeIndex.unwrap();
            let oldEntry = self.entries.offset(by: index).read();
            self.entries.offset(by: index).write(DictionaryEntry(
                key: key,
                value: value,
                hash: UInt64(intLiteral: 0),
                occupied: true
            ));
            return .Some(oldEntry.value)
        }

        // Need to insert - ensure capacity first
        self.ensureCapacity();

        // Find empty slot
        let maybeSlot = self.findEmptySlot();
        if maybeSlot.isSome() {
            let slotIndex = maybeSlot.unwrap();
            self.entries.offset(by: slotIndex).write(DictionaryEntry(
                key: key,
                value: value,
                hash: UInt64(intLiteral: 0),
                occupied: true
            ));
            self.len = self.len + Int64(intLiteral: 1)
        } else {
            lang.panic("Dictionary insert failed - no empty slot")
        }

        let none: Optional[V] = .None;
        none
    }

    // Remove key and return its value
    public mutating func remove(key: K) -> Optional[V] {
        let maybeIndex = self.findEntry(key);

        if maybeIndex.isSome() {
            let index = maybeIndex.unwrap();
            let entry = self.entries.offset(by: index).read();
            let removedValue = entry.value;

            // Mark as unoccupied (keep key/value as placeholder)
            self.entries.offset(by: index).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: UInt64(intLiteral: 0),
                occupied: false
            ));
            self.len = self.len - Int64(intLiteral: 1);

            return .Some(removedValue)
        }

        let none: Optional[V] = .None;
        none
    }

    // Clear all entries
    public mutating func clear() {
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.cap {
            let entry = self.entries.offset(by: i).read();
            // Keep key/value but mark unoccupied
            self.entries.offset(by: i).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: UInt64(intLiteral: 0),
                occupied: false
            ));
            i = i + Int64(intLiteral: 1)
        }
        self.len = Int64(intLiteral: 0)
    }

    // Iteration
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(entries: self.entries, capacity: self.cap)
    }

    // Get internal data for views
    public func getEntries() -> Pointer[DictionaryEntry[K, V]] { self.entries }
}

// Equatable when K and V are Equatable
extend Dictionary[K, V]: Equatable where K: Equatable, V: Equatable {
    public func equals(other: Dictionary[K, V]) -> Bool {
        let selfCount = self.count();
        let otherCount = other.count();
        if selfCount != otherCount {
            return false
        }

        // Check all entries in self exist in other with same value
        var equal: Bool = true;
        var i: Int64 = Int64(intLiteral: 0);
        let selfCap = self.getCapacity();
        let selfEntries = self.getEntries();
        while i < selfCap and equal {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                let otherValue = other.getValue(entry.key);
                if otherValue.isSome() {
                    if entry.value.equals(otherValue.unwrap()) == false {
                        equal = false
                    }
                } else {
                    equal = false
                }
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }
}

// Cloneable when K and V are Cloneable
extend Dictionary[K, V]: Cloneable where K: Cloneable, V: Cloneable {
    public func clone() -> Dictionary[K, V] {
        let selfCount = self.count();
        let selfCap = self.getCapacity();
        let selfEntries = self.getEntries();

        if selfCount == Int64(intLiteral: 0) {
            return Dictionary(self.placeholderKey.clone(), self.placeholderValue.clone())
        }

        // Find first entry to use as placeholder
        var firstKey: K = selfEntries.offset(by: Int64(intLiteral: 0)).read().key;
        var firstValue: V = selfEntries.offset(by: Int64(intLiteral: 0)).read().value;
        var i: Int64 = Int64(intLiteral: 0);
        var foundFirst: Bool = false;
        while i < selfCap and foundFirst == false {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                firstKey = entry.key;
                firstValue = entry.value;
                foundFirst = true
            }
            i = i + Int64(intLiteral: 1)
        }

        var result = Dictionary(capacity: selfCount, placeholderKey: firstKey.clone(), placeholderValue: firstValue.clone());
        i = Int64(intLiteral: 0);
        while i < selfCap {
            let entry = selfEntries.offset(by: i).read();
            if entry.occupied {
                let _ = result.insert(entry.key.clone(), entry.value.clone());
            }
            i = i + Int64(intLiteral: 1)
        }
        result
    }
}

// KeysIterator must be defined before KeysView
public struct KeysIterator[K, V]: Iterator where K: Equatable {
    type Item = K

    private var dictIter: DictionaryIterator[K, V]

    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    public mutating func next() -> Optional[K] {
        let maybeEntry = self.dictIter.next();
        if maybeEntry.isSome() {
            .Some(maybeEntry.unwrap().key)
        } else {
            .None
        }
    }
}

// KeysView - view of dictionary keys
public struct KeysView[K, V]: Iterable where K: Equatable {
    type Item = K
    type Iter = KeysIterator[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64

    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
    }

    public func iter() -> KeysIterator[K, V] {
        KeysIterator(DictionaryIterator(entries: self.entries, capacity: self.capacity))
    }
}

// ValuesIterator must be defined before ValuesView
public struct ValuesIterator[K, V]: Iterator where K: Equatable {
    type Item = V

    private var dictIter: DictionaryIterator[K, V]

    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter;
    }

    public mutating func next() -> Optional[V] {
        let maybeEntry = self.dictIter.next();
        if maybeEntry.isSome() {
            .Some(maybeEntry.unwrap().value)
        } else {
            .None
        }
    }
}

// ValuesView - view of dictionary values
public struct ValuesView[K, V]: Iterable where K: Equatable {
    type Item = V
    type Iter = ValuesIterator[K, V]

    private var entries: Pointer[DictionaryEntry[K, V]]
    private var capacity: Int64

    public init(entries entries: Pointer[DictionaryEntry[K, V]], capacity capacity: Int64) {
        self.entries = entries;
        self.capacity = capacity;
    }

    public func iter() -> ValuesIterator[K, V] {
        ValuesIterator(DictionaryIterator(entries: self.entries, capacity: self.capacity))
    }
}

// Extension to add keys() and values() methods
extend Dictionary[K, V] where K: Equatable {
    public func keys() -> KeysView[K, V] {
        KeysView(entries: self.entries, capacity: self.cap)
    }

    public func values() -> ValuesView[K, V] {
        ValuesView(entries: self.entries, capacity: self.cap)
    }
}
