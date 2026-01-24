// Set[T] - hash set backed by Dictionary

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Dictionary, DictionaryEntry, DictionaryIterator, DefaultHasher)

// Unit type for dictionary values (set only cares about keys)
struct Unit: Equatable, Cloneable {
    init() {}

    func equals(other: Unit) -> Bool { true }
    func clone() -> Unit { Unit() }
}

// SetIterator extracts keys from dictionary iterator
public struct SetIterator[T, H = DefaultHasher]: Iterator where T: Hash, H: Hasher, H: Defaultable {
    type Item = T

    private var dictIter: DictionaryIterator[T, Unit]

    public init(dictIter dictIter: DictionaryIterator[T, Unit]) {
        self.dictIter = dictIter;
    }

    public mutating func next() -> Optional[T] {
        let maybeEntry = self.dictIter.next();
        if maybeEntry.isSome() {
            .Some(maybeEntry.unwrap().key)
        } else {
            .None
        }
    }
}

// Set[T, H] - hash set using Dictionary internally
public struct Set[T, H = DefaultHasher]: Iterable where T: Hash, H: Hasher, H: Defaultable {
    type Item = T
    type Iter = SetIterator[T, H]

    var dict: Dictionary[T, Unit, H]
    var placeholder: T

    // Create empty set - requires placeholder element for dictionary
    public init(placeholder placeholder: T) {
        self.dict = Dictionary(placeholder, Unit());
        self.placeholder = placeholder;
    }

    // Create with initial capacity
    public init(capacity capacity: Int64, placeholder placeholder: T) {
        self.dict = Dictionary(capacity: capacity, placeholderKey: placeholder, placeholderValue: Unit());
        self.placeholder = placeholder;
    }

    // Properties
    public func count() -> Int64 { self.dict.count() }
    public func getCapacity() -> Int64 { self.dict.getCapacity() }
    public func isEmpty() -> Bool { self.dict.isEmpty() }

    // Check if element exists
    public func contains(element: T) -> Bool {
        self.dict.contains(element)
    }

    // Insert element, returns true if element was new
    public mutating func insert(element: T) -> Bool {
        let oldValue = self.dict.insert(element, Unit());
        oldValue.isSome() == false
    }

    // Remove element, returns true if element was present
    public mutating func remove(element: T) -> Bool {
        self.dict.remove(element).isSome()
    }

    // Clear all entries
    public mutating func clear() {
        self.dict.clear()
    }

    // Iteration
    public func iter() -> SetIterator[T, H] {
        SetIterator(dictIter: self.dict.iter())
    }

    // Get internal dictionary for extensions
    func getDict() -> Dictionary[T, Unit, H] { self.dict }

    // Get placeholder for creating new sets
    func getPlaceholder() -> T { self.placeholder }

    // Set operations

    // Union: elements in either set
    public func union(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count();
        let otherCount = other.count();

        var result = Set(capacity: selfCount + otherCount, placeholder: self.placeholder);

        // Add all from self
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() {
            let _ = result.insert(maybeElem.unwrap());
            maybeElem = selfIter.next()
        }

        // Add all from other
        var otherIter = other.iter();
        maybeElem = otherIter.next();
        while maybeElem.isSome() {
            let _ = result.insert(maybeElem.unwrap());
            maybeElem = otherIter.next()
        }

        result
    }

    // Intersection: elements in both sets
    public func intersection(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count();

        var result = Set(capacity: selfCount, placeholder: self.placeholder);

        // Add elements that are in both
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() {
            let elem = maybeElem.unwrap();
            if other.contains(elem) {
                let _ = result.insert(elem);
            }
            maybeElem = selfIter.next()
        }

        result
    }

    // Difference: elements in self but not in other
    public func difference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count();

        var result = Set(capacity: selfCount, placeholder: self.placeholder);

        // Add elements not in other
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() {
            let elem = maybeElem.unwrap();
            if other.contains(elem) == false {
                let _ = result.insert(elem);
            }
            maybeElem = selfIter.next()
        }

        result
    }

    // Symmetric difference: elements in either but not both
    public func symmetricDifference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count();
        let otherCount = other.count();

        var result = Set(capacity: selfCount + otherCount, placeholder: self.placeholder);

        // Add elements in self but not other
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() {
            let elem = maybeElem.unwrap();
            if other.contains(elem) == false {
                let _ = result.insert(elem);
            }
            maybeElem = selfIter.next()
        }

        // Add elements in other but not self
        var otherIter = other.iter();
        maybeElem = otherIter.next();
        while maybeElem.isSome() {
            let elem = maybeElem.unwrap();
            if self.contains(elem) == false {
                let _ = result.insert(elem);
            }
            maybeElem = otherIter.next()
        }

        result
    }

    // Subset check: all elements in self are in other
    public func isSubset(other: Set[T, H]) -> Bool {
        var allFound: Bool = true;
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() and allFound {
            if other.contains(maybeElem.unwrap()) == false {
                allFound = false
            }
            maybeElem = selfIter.next()
        }
        allFound
    }

    // Superset check: all elements in other are in self
    public func isSuperset(other: Set[T, H]) -> Bool {
        other.isSubset(self)
    }

    // Disjoint check: no common elements
    public func isDisjoint(other: Set[T, H]) -> Bool {
        var noCommon: Bool = true;
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() and noCommon {
            if other.contains(maybeElem.unwrap()) {
                noCommon = false
            }
            maybeElem = selfIter.next()
        }
        noCommon
    }

    // Cloneable - Set gets COW semantics from Dictionary
    public func clone() -> Set[T, H] {
        Set(dict: self.dict.clone(), placeholder: self.placeholder)
    }

    // Private init for clone
    private init(dict dict: Dictionary[T, Unit, H], placeholder placeholder: T) {
        self.dict = dict;
        self.placeholder = placeholder;
    }
}

// Equatable extension
extend Set[T, H]: Equatable where T: Hash, H: Hasher, H: Defaultable {
    public func equals(other: Set[T, H]) -> Bool {
        let selfCount = self.count();
        let otherCount = other.count();
        if selfCount != otherCount {
            return false
        }

        // Check all elements in self exist in other
        var equal: Bool = true;
        var selfIter = self.iter();
        var maybeElem = selfIter.next();
        while maybeElem.isSome() and equal {
            if other.contains(maybeElem.unwrap()) == false {
                equal = false
            }
            maybeElem = selfIter.next()
        }
        equal
    }
}

// Cloneable extension
extend Set[T, H]: Cloneable where T: Hash, H: Hasher, H: Defaultable {}
