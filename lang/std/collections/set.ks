// Set[T] - hash set backed by Dictionary

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Dictionary, DictionaryEntry, DictionaryIterator, DefaultHasher)

// ============================================================================
// INTERNAL TYPES
// ============================================================================

/// Unit type for dictionary values (set only cares about keys).
struct Unit: Equatable, Cloneable {
    init() {}

    func equals(other: Unit) -> Bool { true }
    func clone() -> Unit { Unit() }
}

// ============================================================================
// SET ITERATOR
// ============================================================================

/// Iterator over set elements.
public struct SetIterator[T, H = DefaultHasher]: Iterator where T: Hash, H: Hasher, H: Defaultable {
    type Item = T

    private var dictIter: DictionaryIterator[T, Unit]

    /// Creates a set iterator.
    public init(dictIter dictIter: DictionaryIterator[T, Unit]) {
        self.dictIter = dictIter;
    }

    /// Returns the next element, or None if exhausted.
    public mutating func next() -> Optional[T] {
        let maybeEntry = self.dictIter.next();
        if maybeEntry.isSome() {
            .Some(maybeEntry.unwrap().key)
        } else {
            .None
        }
    }
}

// ============================================================================
// SET
// ============================================================================

/// A hash set backed by a Dictionary.
///
/// Elements must implement Hash. Uses Dictionary internally with Unit values.
public struct Set[T, H = DefaultHasher]: Iterable, Cloneable where T: Hash, H: Hasher, H: Defaultable {
    type Item = T
    type Iter = SetIterator[T, H]

    var dict: Dictionary[T, Unit, H]
    var placeholder: T

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an empty set.
    ///
    /// Requires placeholder element for internal dictionary storage.
    public init(placeholder placeholder: T) {
        self.dict = Dictionary(placeholder, Unit());
        self.placeholder = placeholder;
    }

    /// Creates an empty set with initial capacity.
    public init(capacity capacity: Int64, placeholder placeholder: T) {
        self.dict = Dictionary(capacity: capacity, placeholderKey: placeholder, placeholderValue: Unit());
        self.placeholder = placeholder;
    }

    // ========================================================================
    // SIZE & CAPACITY
    // ========================================================================

    /// The number of elements in the set.
    public func count() -> Int64 { self.dict.count() }

    /// The allocated capacity.
    public func getCapacity() -> Int64 { self.dict.getCapacity() }

    /// True if the set is empty.
    public func isEmpty() -> Bool { self.dict.isEmpty() }

    // ========================================================================
    // MEMBERSHIP
    // ========================================================================

    /// Returns true if the set contains the element.
    public func contains(element: T) -> Bool {
        self.dict.contains(element)
    }

    // ========================================================================
    // MUTATING OPERATIONS
    // ========================================================================

    /// Inserts an element. Returns true if the element was new.
    public mutating func insert(element: T) -> Bool {
        let oldValue = self.dict.insert(element, Unit());
        oldValue.isSome() == false
    }

    /// Removes an element. Returns true if the element was present.
    public mutating func remove(element: T) -> Bool {
        self.dict.remove(element).isSome()
    }

    /// Removes all elements from the set.
    public mutating func clear() {
        self.dict.clear()
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the elements.
    public func iter() -> SetIterator[T, H] {
        SetIterator(dictIter: self.dict.iter())
    }

    /// Returns the internal dictionary (for extensions).
    func getDict() -> Dictionary[T, Unit, H] { self.dict }

    /// Returns the placeholder (for creating new sets).
    func getPlaceholder() -> T { self.placeholder }

    // ========================================================================
    // SET OPERATIONS
    // ========================================================================

    /// Returns the union of two sets (elements in either set).
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

    /// Returns the intersection of two sets (elements in both sets).
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

    /// Returns the difference of two sets (elements in self but not in other).
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

    /// Returns the symmetric difference (elements in either but not both).
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

    // ========================================================================
    // SET PREDICATES
    // ========================================================================

    /// Returns true if all elements of self are in other.
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

    /// Returns true if all elements of other are in self.
    public func isSuperset(other: Set[T, H]) -> Bool {
        other.isSubset(self)
    }

    /// Returns true if the sets have no common elements.
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

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Creates a shallow clone (COW - copy deferred until mutation).
    public func clone() -> Set[T, H] {
        Set(dict: self.dict.clone(), placeholder: self.placeholder)
    }

    /// Private init for clone.
    private init(dict dict: Dictionary[T, Unit, H], placeholder placeholder: T) {
        self.dict = dict;
        self.placeholder = placeholder;
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// Equatable extension.
extend Set[T, H]: Equatable where T: Hash, H: Hasher, H: Defaultable {
    /// Compares two sets for equality.
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

