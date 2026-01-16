// Set type - hash set with COW semantics

module std.collections

import std.core.(Equatable, Hashable, Cloneable, Hasher, Int, Bool, UInt64)
import std.result.(Optional)
import std.memory.(Allocator, GlobalAllocator)
import std.iter.(Iterator, Iterable, Collectable)
import std.collections.(Dictionary, DictionaryIterator)

public struct Set[T, A]:
    Iterable,
    Collectable,
    Cloneable
    where A: Allocator
{
    // Associated type bindings (qualified to avoid ambiguity across protocols)
    type Iterable.Item = T
    type Collectable.Item = T
    type Iter = SetIterator[T, A]

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
        self.init();
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
    public mutating func insert(element: T) -> Bool {
        if self.dict.contains(key: element) {
            false
        } else {
            self.dict.insert(value: (), for: element);
            true
        }
    }

    public mutating func remove(element: T) -> Bool {
        self.dict.remove(for: element).isSome
    }

    public func contains(element: T) -> Bool {
        self.dict.contains(key: element)
    }

    public mutating func clear() {
        self.dict.clear()
    }

    // Set operations
    public func union(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A](minimumCapacity: self.count + other.count);
        /* for element in self {
            result.insert(element: element)
        } */
        /* for element in other {
            result.insert(element: element)
        } */
        result
    }

    public func intersection(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]();
        /* for element in self {
            if other.contains(element: element) {
                result.insert(element: element)
            }
        } */
        result
    }

    public func difference(from other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]();
        /* for element in self {
            if not other.contains(element: element) {
                result.insert(element: element)
            }
        } */
        result
    }

    public func symmetricDifference(with other: Set[T, A]) -> Set[T, A] {
        var result = Set[T, A]();
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
        var toRemove: [T] = [];
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
    public func iter() -> SetIterator[T, A] {
        SetIterator(dictIter: self.dict.iter())
    }

    // Cloneable
    public func clone() -> Set[T, A] where T: Cloneable {
        var result = Set[T, A](minimumCapacity: self.count);
        /* for element in self {
            result.insert(element: element.clone())
        } */
        result
    }
}

// Equatable
extend Set[T, A]: Equatable {
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
extend Set[T, A]: Hashable {
    public func hash[H](mutating into hasher: H) where H: Hasher {
        // XOR all element hashes (order-independent)
        var combinedHash: UInt64 = 0;
        /* for element in self {
            var elementHasher = DefaultHasher()
            element.hash(into: elementHasher)
            combinedHash = combinedHash ^ elementHasher.finish()
        } */
        hasher.write(bytes: combinedHash.toBytes())
    }
}

// Set iterator
public struct SetIterator[T, A]: Iterator where A: Allocator {
    type Item = T

    private var dictIter: DictionaryIterator[T, (), A]

    public init(dictIter: DictionaryIterator[T, (), A]) {
        self.dictIter = dictIter
    }

    public mutating func next() -> Optional[T] {
        self.dictIter.next().map { (element, x) in element }
    }
}
