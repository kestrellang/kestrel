// Dictionary[K, V] - hash map with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable)
import std.num.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, SystemAllocator, RcBox)
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

// DictionaryStorage[K, V] - internal storage for Dictionary
struct DictionaryStorage[K, V]: Cloneable where K: Equatable {
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

    // Deep clone - allocate new buffer and copy entries
    func clone() -> DictionaryStorage[K, V] {
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
        if result.isSome() {
            let newEntries = result.unwrap().cast[DictionaryEntry[K, V]]();
            // Copy entries
            var i: Int64 = Int64(intLiteral: 0);
            while i < self.cap {
                newEntries.offset(by: i).write(self.entries.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
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

// Dictionary[K, V] - hash map with COW semantics using RcBox
public struct Dictionary[K, V]: Iterable where K: Equatable {
    type Item = DictionaryEntry[K, V]
    type Iter = DictionaryIterator[K, V]

    private var storage: RcBox[DictionaryStorage[K, V]]

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

    // Private init for internal use (from storage)
    private init(storage storage: RcBox[DictionaryStorage[K, V]]) {
        self.storage = storage;
    }

    // Create empty dictionary - requires placeholder key/value for future resizing
    public init(placeholderKey: K, placeholderValue: V) {
        self.storage = RcBox(DictionaryStorage(
            entries: Pointer(raw: lang.ptr_null[DictionaryEntry[K, V]]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0),
            placeholderKey: placeholderKey,
            placeholderValue: placeholderValue
        ));
    }

    // Create with initial capacity - requires a placeholder key/value for initialization
    public init(capacity capacity: Int64, placeholderKey placeholderKey: K, placeholderValue placeholderValue: V) {
        let actualCap = nextPowerOfTwo(capacity);
        if actualCap > Int64(intLiteral: 0) {
            let layout = Layout.array[DictionaryEntry[K, V]](actualCap);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if result.isSome() {
                let newEntries = result.unwrap().cast[DictionaryEntry[K, V]]();
                // Initialize all entries as unoccupied
                var i: Int64 = Int64(intLiteral: 0);
                while i < actualCap {
                    newEntries.offset(by: i).write(DictionaryEntry(
                        placeholderKey: placeholderKey,
                        placeholderValue: placeholderValue
                    ));
                    i = i + Int64(intLiteral: 1)
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

    // Simple hash function - XOR shift based on the UInt64 representation
    // This works for types whose first 8 bytes are meaningful (Int64, UInt64, pointers, etc.)
    // For proper hashing, types should implement Hashable protocol
    private func hashKey(key: K) -> UInt64 {
        // Use a constant hash for now (all keys hash to same bucket - O(n) but correct)
        // TODO: Implement proper hashing when Hashable protocol is complete
        UInt64(intLiteral: 0)
    }

    // Properties
    public func count() -> Int64 { self.len() }
    public func getCapacity() -> Int64 { self.cap() }
    public func isEmpty() -> Bool { self.len() == Int64(intLiteral: 0) }

    // Find entry by key using linear search (since hash is constant)
    private func findEntry(key: K) -> Optional[Int64] {
        let myCap = self.cap();
        if myCap == Int64(intLiteral: 0) {
            let none: Optional[Int64] = .None;
            return none
        }

        let myEntries = self.entries();
        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < myCap and done == false {
            let entry = myEntries.offset(by: i).read();
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
        let myCap = self.cap();
        let myEntries = self.entries();
        var i: Int64 = Int64(intLiteral: 0);
        var result: Optional[Int64] = .None;
        var done: Bool = false;

        while i < myCap and done == false {
            let entry = myEntries.offset(by: i).read();
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
        let myCap = self.cap();
        let myLen = self.len();
        let threshold = myCap * Int64(intLiteral: 3) / Int64(intLiteral: 4);
        if myLen >= threshold or myCap == Int64(intLiteral: 0) {
            self.resize()
        }
    }

    // Resize the hash table
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
        if result.isSome() {
            let newEntries = result.unwrap().cast[DictionaryEntry[K, V]]();

            // Initialize new entries
            var i: Int64 = Int64(intLiteral: 0);
            while i < newCap {
                newEntries.offset(by: i).write(DictionaryEntry(
                    placeholderKey: s.placeholderKey,
                    placeholderValue: s.placeholderValue
                ));
                i = i + Int64(intLiteral: 1)
            }

            // Copy old entries
            var newLen: Int64 = Int64(intLiteral: 0);
            var slotIndex: Int64 = Int64(intLiteral: 0);
            i = Int64(intLiteral: 0);
            while i < oldCap {
                let entry = oldEntries.offset(by: i).read();
                if entry.occupied {
                    // Find empty slot in new table
                    var foundSlot: Bool = false;
                    while slotIndex < newCap and foundSlot == false {
                        let slotEntry = newEntries.offset(by: slotIndex).read();
                        if slotEntry.occupied == false {
                            newEntries.offset(by: slotIndex).write(entry);
                            newLen = newLen + Int64(intLiteral: 1);
                            foundSlot = true
                        }
                        slotIndex = slotIndex + Int64(intLiteral: 1)
                    }
                    if foundSlot == false {
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

    // Get value for key
    public func getValue(key: K) -> Optional[V] {
        let maybeIndex = self.findEntry(key);
        if maybeIndex.isSome() {
            let entry = self.entries().offset(by: maybeIndex.unwrap()).read();
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
            self.makeUnique();
            let index = maybeIndex.unwrap();
            let myEntries = self.entries();
            let oldEntry = myEntries.offset(by: index).read();
            myEntries.offset(by: index).write(DictionaryEntry(
                key: key,
                value: value,
                hash: UInt64(intLiteral: 0),
                occupied: true
            ));
            return .Some(oldEntry.value)
        }

        // Need to insert - ensure capacity first
        self.ensureCapacity();
        self.makeUnique();

        // Find empty slot
        let maybeSlot = self.findEmptySlot();
        if maybeSlot.isSome() {
            var s = self.storage.getValue();
            let slotIndex = maybeSlot.unwrap();
            s.entries.offset(by: slotIndex).write(DictionaryEntry(
                key: key,
                value: value,
                hash: UInt64(intLiteral: 0),
                occupied: true
            ));
            s.len = s.len + Int64(intLiteral: 1);
            self.storage.setValue(s)
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
            self.makeUnique();
            var s = self.storage.getValue();
            let index = maybeIndex.unwrap();
            let entry = s.entries.offset(by: index).read();
            let removedValue = entry.value;

            // Mark as unoccupied (keep key/value as placeholder)
            s.entries.offset(by: index).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: UInt64(intLiteral: 0),
                occupied: false
            ));
            s.len = s.len - Int64(intLiteral: 1);
            self.storage.setValue(s);

            return .Some(removedValue)
        }

        let none: Optional[V] = .None;
        none
    }

    // Clear all entries
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        var i: Int64 = Int64(intLiteral: 0);
        while i < s.cap {
            let entry = s.entries.offset(by: i).read();
            // Keep key/value but mark unoccupied
            s.entries.offset(by: i).write(DictionaryEntry(
                key: entry.key,
                value: entry.value,
                hash: UInt64(intLiteral: 0),
                occupied: false
            ));
            i = i + Int64(intLiteral: 1)
        }
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    // Iteration
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(entries: self.entries(), capacity: self.cap())
    }

    // Get internal data for views
    public func getEntries() -> Pointer[DictionaryEntry[K, V]] { self.entries() }

    // Cloneable - shallow clone (COW)
    public func clone() -> Dictionary[K, V] {
        Dictionary(storage: self.storage.clone())
    }
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

// Cloneable conformance
extend Dictionary[K, V]: Cloneable {}

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
        KeysView(entries: self.getEntries(), capacity: self.getCapacity())
    }

    public func values() -> ValuesView[K, V] {
        ValuesView(entries: self.getEntries(), capacity: self.getCapacity())
    }
}
