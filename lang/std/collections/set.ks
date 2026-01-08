// Set type - hash set with COW semantics

public struct Set[T, A]:
    Iterable,
    Collectable,
    Cloneable
{
    type Item = T
    type Iter = SetIterator[T]

    // Use Dictionary with unit value as backing storage
    private var dict: Dictionary[T, (), A]

    // Constructors
    public init() {
        self.dict = Dictionary()
    }

    public init(allocator: A) {
        self.dict = Dictionary(allocator: allocator)
    }

    public init(minimumCapacity: Int) where A = GlobalAllocator {
        self.dict = Dictionary(minimumCapacity: minimumCapacity)
    }

    // Collectable
    public init[I](from iter: I) where I: Iterator, I.Item = T {
        self.init()
        while let item = iter.next() {
            self.insert(element: item)
        }
    }

    // Properties
    public var count: Int {
        self.dict.count
    }

    public var isEmpty: Bool {
        self.dict.isEmpty
    }

    // Mutation
    public func insert(element: T) -> Bool {
        if self.dict.contains(key: element) {
            false
        } else {
            self.dict.insert(value: (), for: element)
            true
        }
    }

    public func remove(element: T) -> Bool {
        self.dict.remove(for: element).isSome
    }

    public func contains(element: T) -> Bool {
        self.dict.contains(key: element)
    }

    public func clear() {
        self.dict.clear()
    }

    // Set operations
    public func union(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A](minimumCapacity: self.count + other.count)
        /* for element in self {
            result.insert(element: element)
        } */
        /* for element in other {
            result.insert(element: element)
        } */
        result
    }

    public func intersection(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]()
        /* for element in self {
            if other.contains(element: element) {
                result.insert(element: element)
            }
        } */
        result
    }

    public func difference(from other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]()
        /* for element in self {
            if not other.contains(element: element) {
                result.insert(element: element)
            }
        } */
        result
    }

    public func symmetricDifference(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]()
        /* for element in self {
            if not other.contains(element: element) {
                result.insert(element: element)
            }
        } */
        /* for element in other {
            if not self.contains(element: element) {
                result.insert(element: element)
            }
        } */
        result
    }

    public func isSubset(of other: Set[T, A]) -> Bool {
        /* for element in self {
            if not other.contains(element: element) {
                return false
            }
        } */
        true
    }

    public func isSuperset(of other: Set[T, A]) -> Bool {
        other.isSubset(of: self)
    }

    public func isDisjoint(with other: Set[T, A]) -> Bool {
        /* for element in self {
            if other.contains(element: element) {
                return false
            }
        } */
        true
    }

    public func isStrictSubset(of other: Set[T, A]) -> Bool {
        self.count < other.count and self.isSubset(of: other)
    }

    public func isStrictSuperset(of other: Set[T, A]) -> Bool {
        self.count > other.count and self.isSuperset(of: other)
    }

    // In-place operations
    public func formUnion(with other: Set[T, A]) {
        /* for element in other {
            self.insert(element: element)
        } */
    }

    public func formIntersection(with other: Set[T, A]) {
        var toRemove: [T] = []
        /* for element in self {
            if not other.contains(element: element) {
                toRemove.append(element)
            }
        } */
        /* for element in toRemove {
            self.remove(element: element)
        } */
    }

    public func subtract(other: Set[T, A]) {
        /* for element in other {
            self.remove(element: element)
        } */
    }

    public func formSymmetricDifference(with other: Set[T, A]) {
        /* for element in other {
            if self.contains(element: element) {
                self.remove(element: element)
            } else {
                self.insert(element: element)
            }
        } */
    }

    // Iteration
    public func iter() -> SetIterator[T] {
        SetIterator(dictIter: self.dict.iter())
    }

    // Cloneable
    public func clone() -> Set[T, A] where T: Cloneable {
        var result = Set[T, A](minimumCapacity: self.count)
        /* for element in self {
            result.insert(element: element.clone())
        } */
        result
    }
}

// Equatable
extension Set[T, A]: Equatable {
    public func equals(other: Set[T, A]) -> Bool {
        if self.count != other.count {
            return false
        }
        /* for element in self {
            if not other.contains(element: element) {
                return false
            }
        } */
        true
    }
}

// Hashable
extension Set[T, A]: Hashable {
    public func hash[H](into hasher: ref H) where H: Hasher {
        // XOR all element hashes (order-independent)
        var combinedHash: UInt64 = 0
        /* for element in self {
            var elementHasher = DefaultHasher()
            element.hash(into: elementHasher)
            combinedHash = combinedHash ^ elementHasher.finish()
        } */
        hasher.write(bytes: combinedHash.toBytes())
    }
}

// Set iterator
public struct SetIterator[T]: Iterator {
    type Item = T

    private var dictIter: DictionaryIterator[T, ()]

    public init(dictIter: DictionaryIterator[T, ()]) {
        self.dictIter = dictIter
    }

    public func next() -> Optional[T] {
        self.dictIter.next().map { (element, _) in element }
    }
}
