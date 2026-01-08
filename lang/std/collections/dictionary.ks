// Dictionary type - hash map with COW semantics

public struct Dictionary[K, V, A]:
    Iterable,
    ExpressibleByDictionaryLiteral,
    Cloneable
    where A: Allocator
{
    type Item = (K, V)
    type Key = K
    type Value = V
    type Iter = DictionaryIterator[K, V]

    private var storage: ArcBox[DictionaryStorage[K, V, A]]

    struct Entry[K, V] {
        var key: K
        var value: V
        var hash: UInt64
        var occupied: Bool
    }

    struct DictionaryStorage[K, V, A] where A: Allocator {
        var entries: Buffer[Entry[K, V], A]
        var count: Int
        var capacity: Int
    }

    // Constructors
    public init() {
        self.storage = ArcBox(value: DictionaryStorage(
            entries: Buffer(capacity: 0),
            count: 0,
            capacity: 0
        ))
    }

    public init(allocator: A) {
        self.storage = ArcBox(value: DictionaryStorage(
            entries: Buffer(capacity: 0, allocator: allocator),
            count: 0,
            capacity: 0
        ))
    }

    public init(minimumCapacity: Int) {
        let capacity = Self.nextPowerOfTwo(minimumCapacity)
        self.storage = ArcBox(value: DictionaryStorage(
            entries: Buffer(capacity: capacity),
            count: 0,
            capacity: capacity
        ))
        self.initializeEntries()
    }

    // ExpressibleByDictionaryLiteral
    public init(dictionaryLiteral pairs: [(K, V)]) {
        self.init(minimumCapacity: pairs.count)
        /* for (key, value) in pairs {
            self.insert(value: value, for: key)
        } */
    }

    private static func nextPowerOfTwo(n: Int) -> Int {
        var p = 1
        while p < n {
            p = p * 2
        }
        if p < 8 { 8 } else { p }
    }

    private func initializeEntries() {
        /* for i in 0..<self.storage.value.capacity {
            self.storage.value.entries(unchecked: i) = Entry(
                key: lang.uninitialized[K](),
                value: lang.uninitialized[V](),
                hash: 0,
                occupied: false
            )
        } */
    }

    // Properties
    public var count: Int {
        self.storage.value.count
    }

    public var isEmpty: Bool {
        self.storage.value.count == 0
    }

    public var keys: KeysView[K, V, A] {
        KeysView(dict: self)
    }

    public var values: ValuesView[K, V, A] {
        ValuesView(dict: self)
    }

    // COW helper
    private func ensureUnique() {
        if not self.storage.isUnique() {
            self.storage = self.storage.deepClone()
        }
    }

    private func ensureCapacity() {
        // Resize when load factor > 0.75
        if self.storage.value.count * 4 >= self.storage.value.capacity * 3 {
            self.resize()
        }
    }

    private func resize() {
        let newCapacity = if self.storage.value.capacity == 0 { 8 } else { self.storage.value.capacity * 2 }
        let oldEntries = self.storage.value.entries
        let oldCapacity = self.storage.value.capacity

        self.storage.value.entries = Buffer(capacity: newCapacity)
        self.storage.value.capacity = newCapacity
        self.storage.value.count = 0
        self.initializeEntries()

        // Rehash all entries
        /* for i in 0..<oldCapacity {
            let entry = oldEntries(unchecked: i)
            if entry.occupied {
                self.insertEntry(key: entry.key, value: entry.value, hash: entry.hash)
            }
        } */
    }

    private func hash(key: K) -> UInt64 {
        var hasher = DefaultHasher()
        key.hash(into: hasher)
        hasher.finish()
    }

    private func findSlot(hash: UInt64) -> Int {
        let mask = self.storage.value.capacity - 1
        var index = Int(hash) & mask

        while self.storage.value.entries(unchecked: index).occupied {
            if self.storage.value.entries(unchecked: index).hash == hash {
                return index
            }
            index = (index + 1) & mask
        }
        index
    }

    private func findEntry(key: K, hash: UInt64) -> Optional[Int] {
        if self.storage.value.capacity == 0 {
            return .None
        }

        let mask = self.storage.value.capacity - 1
        var index = Int(hash) & mask
        var checked = 0

        while checked < self.storage.value.capacity {
            let entry = self.storage.value.entries(unchecked: index)
            if not entry.occupied {
                return .None
            }
            if entry.hash == hash and entry.key == key {
                return .Some(index)
            }
            index = (index + 1) & mask
            checked += 1
        }
        .None
    }

    private func insertEntry(key: K, value: V, hash: UInt64) {
        let mask = self.storage.value.capacity - 1
        var index = Int(hash) & mask

        while self.storage.value.entries(unchecked: index).occupied {
            index = (index + 1) & mask
        }

        self.storage.value.entries(unchecked: index) = Entry(
            key: key,
            value: value,
            hash: hash,
            occupied: true
        )
        self.storage.value.count += 1
    }

    // Subscript access
    public subscript(key: K) -> Optional[V] {
        get {
            let hash = self.hash(key: key)
            if let index = self.findEntry(key: key, hash: hash) {
                .Some(self.storage.value.entries(unchecked: index).value)
            } else {
                .None
            }
        }
        set {
            self.ensureUnique()
            if let value = newValue {
                self.insert(value: value, for: key)
            } else {
                self.remove(for: key)
            }
        }
    }

    // Mutation
    public func insert(value: V, for key: K) -> Optional[V] {
        self.ensureUnique()
        let hash = self.hash(key: key)

        // Check if key exists
        if let index = self.findEntry(key: key, hash: hash) {
            let oldValue = self.storage.value.entries(unchecked: index).value
            self.storage.value.entries(unchecked: index).value = value
            return .Some(oldValue)
        }

        // Insert new entry
        self.ensureCapacity()
        self.insertEntry(key: key, value: value, hash: hash)
        .None
    }

    public func remove(for key: K) -> Optional[V] {
        self.ensureUnique()
        let hash = self.hash(key: key)

        if let index = self.findEntry(key: key, hash: hash) {
            let value = self.storage.value.entries(unchecked: index).value
            self.storage.value.entries(unchecked: index).occupied = false
            self.storage.value.count -= 1

            // Rehash following entries (linear probing requires this)
            let mask = self.storage.value.capacity - 1
            var i = (index + 1) & mask
            while self.storage.value.entries(unchecked: i).occupied {
                let entry = self.storage.value.entries(unchecked: i)
                self.storage.value.entries(unchecked: i).occupied = false
                self.storage.value.count -= 1
                self.insertEntry(key: entry.key, value: entry.value, hash: entry.hash)
                i = (i + 1) & mask
            }

            return .Some(value)
        }
        .None
    }

    public func contains(key: K) -> Bool {
        let hash = self.hash(key: key)
        self.findEntry(key: key, hash: hash).isSome
    }

    public func clear() {
        self.ensureUnique()
        /* for i in 0..<self.storage.value.capacity {
            self.storage.value.entries(unchecked: i).occupied = false
        } */
        self.storage.value.count = 0
    }

    // Iteration
    public func iter() -> DictionaryIterator[K, V] {
        DictionaryIterator(dict: self, index: 0)
    }

    // Cloneable
    public func clone() -> Dictionary[K, V, A] where K: Cloneable, V: Cloneable {
        var result = Dictionary[K, V, A](minimumCapacity: self.count)
        /* for (key, value) in self {
            result.insert(value: value.clone(), for: key.clone())
        } */
        result
    }

    // Get or insert
    public func getOrInsert(key: K, default defaultValue: V) -> V {
        if let value = self[key] {
            return value
        }
        self.insert(value: defaultValue, for: key)
        defaultValue
    }

    public func getOrInsertWith(key: K, defaultFn: () -> V) -> V {
        if let value = self[key] {
            return value
        }
        let value = defaultFn()
        self.insert(value: value, for: key)
        value
    }
}

// Equatable when K and V are Equatable
extension Dictionary[K, V, A]: Equatable where K: Equatable, V: Equatable {
    public func equals(other: Dictionary[K, V, A]) -> Bool {
        if self.count != other.count {
            return false
        }
        /* for (key, value) in self {
            match other[key] {
                .Some(let otherValue) => {
                    if value != otherValue {
                        return false
                    }
                },
                .None => return false
            }
        } */
        true
    }
}

// Dictionary iterator
public struct DictionaryIterator[K, V]: Iterator {
    type Item = (K, V)

    private var dict: Dictionary[K, V]
    private var index: Int

    public init(dict: Dictionary[K, V], index: Int) {
        self.dict = dict;
        self.index = index;
    }

    public func next() -> Optional[(K, V)] {
        while self.index < self.dict.storage.value.capacity {
            let entry = self.dict.storage.value.entries(unchecked: self.index)
            self.index += 1
            if entry.occupied {
                return .Some((entry.key, entry.value))
            }
        }
        .None
    }
}

// Keys view
public struct KeysView[K, V, A]: Iterable where A: Allocator {
    type Item = K
    type Iter = KeysIterator[K, V]

    private var dict: Dictionary[K, V, A]

    public init(dict: Dictionary[K, V, A]) {
        self.dict = dict;
    }

    public func iter() -> KeysIterator[K, V] {
        KeysIterator(dictIter: self.dict.iter())
    }
}

public struct KeysIterator[K, V]: Iterator {
    type Item = K

    private var dictIter: DictionaryIterator[K, V]

    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter
    }

    public func next() -> Optional[K] {
        self.dictIter.next().map { (key, _) in key }
    }
}

// Values view
public struct ValuesView[K, V, A]: Iterable where A: Allocator {
    type Item = V
    type Iter = ValuesIterator[K, V]

    private var dict: Dictionary[K, V, A]

    public init(dict: Dictionary[K, V, A]) {
        self.dict = dict;
    }

    public func iter() -> ValuesIterator[K, V] {
        ValuesIterator(dictIter: self.dict.iter())
    }
}

public struct ValuesIterator[K, V]: Iterator {
    type Item = V

    private var dictIter: DictionaryIterator[K, V]

    public init(dictIter: DictionaryIterator[K, V]) {
        self.dictIter = dictIter
    }

    public func next() -> Optional[V] {
        self.dictIter.next().map { (_, value) in value }
    }
}
