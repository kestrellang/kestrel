// Set[T] - hash set with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hash, Hasher, Defaultable, Addable, Comparable)
import std.text.(Formattable, FormatOptions)
import std.num.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Dictionary, DictionaryIterator, DefaultHasher)
import std.memory.(LiteralSlice)
import std.text.(String)
import std.core.(ExpressibleByArrayLiteral)

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

/// Iterator for Set that yields elements sequentially.
///
/// Obtained by calling `iter()` on a set. Typically used implicitly
/// via for-in loops or iterator methods.
///
/// Example:
///     let set: Set = [1, 2, 3]
///     for item in set {
///         print(item)
///     }
public struct SetIterator[T, H = DefaultHasher]: Iterator where T: Hash, H: Hasher, H: Defaultable {
    type Item = T

    private var dictIter: DictionaryIterator[T, Unit]

    /// Creates a set iterator from a dictionary iterator.
    /// Note: This is a low-level initializer; prefer using `set.iter()`.
    public init(dictIter dictIter: DictionaryIterator[T, Unit]) {
        self.dictIter = dictIter;
    }

    /// Returns the next element, or None if exhausted.
    public mutating func next() -> T? {
        let maybePair = self.dictIter.next();
        if let .Some(pair) = maybePair {
            .Some(pair.0)
        } else {
            .None
        }
    }
}

// ============================================================================
// SET
// ============================================================================

/// A hash set that stores unique elements with copy-on-write semantics.
///
/// Sets provide O(1) average-case lookup, insertion, and removal. They use
/// COW for efficient copying - the underlying storage is only duplicated
/// when a shared set is mutated.
///
/// Backed by a Dictionary internally.
///
/// Example:
///     var fruits: Set = ["apple", "banana", "cherry"]
///     fruits.insert( "date")
///     fruits.contains( "apple")  // true
///     fruits.remove( "banana")
///
/// Set literals use array syntax with type annotation:
///     let empty: Set[Int64] = []
///     let numbers: Set = [1, 2, 3]
public struct Set[T, H = DefaultHasher]: Iterable, Cloneable where T: Hash, H: Hasher, H: Defaultable {
    type Item = T
    type Iter = SetIterator[T, H]

    var dict: Dictionary[T, Unit, H]

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// Creates an empty set.
    ///
    /// Example:
    ///     let set = Set[String]()
    ///     set.isEmpty  // true
    public init() {
        self.dict = Dictionary();
    }

    /// Creates an empty set with the specified initial capacity.
    ///
    /// Pre-allocating capacity can improve performance when the approximate
    /// final size is known, avoiding repeated reallocations.
    ///
    /// Example:
    ///     var set = Set[String](capacity: 1000)
    ///     set.capacity  // >= 1000
    ///     set.count     // 0
    public init(capacity capacity: Int64) {
        self.dict = Dictionary(capacity: capacity);
    }

    /// Creates a set from an iterable of elements.
    ///
    /// Duplicate elements are automatically removed.
    ///
    /// Example:
    ///     let arr = [1, 2, 2, 3, 3, 3]
    ///     let set = Set(from: arr)  // {1, 2, 3}
    public init[I](from elements: I) where I: Iterable, I.Item = T {
        self.dict = Dictionary();
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    /// Creates a set from an array literal (used by compiler).
    public init(arrayLiteral elements: LiteralSlice[T]) {
        self.dict = Dictionary(capacity: elements.count());
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Returns the number of elements in the set.
    ///
    /// Example:
    ///     Set([1, 2, 3]).count  // 3
    ///     Set[Int64]().count    // 0
    public var count: Int64 {
        get { self.dict.count }
    }

    /// Returns the current capacity (elements storable without reallocating).
    ///
    /// Capacity is always >= count. When count exceeds capacity, the set
    /// reallocates with increased capacity.
    ///
    /// Example:
    ///     var set = Set[String](capacity: 100)
    ///     set.capacity  // >= 100
    public var capacity: Int64 {
        get { self.dict.capacity }
    }

    /// Returns true if the set contains no elements.
    ///
    /// Example:
    ///     Set[Int64]().isEmpty  // true
    ///     Set([1]).isEmpty      // false
    public var isEmpty: Bool { 
        get { self.dict.isEmpty }
    }

    // ========================================================================
    // MEMBERSHIP
    // ========================================================================

    /// Returns true if the set contains the specified element.
    ///
    /// Example:
    ///     let set: Set = [1, 2, 3]
    ///     set.contains( 2)  // true
    ///     set.contains( 5)  // false
    public func contains(element: T) -> Bool {
        self.dict.contains(element)
    }

    /// Returns an iterator over the set's elements.
    ///
    /// Iteration order is not guaranteed to match insertion order.
    ///
    /// Example:
    ///     for item in set.iter() {
    ///         print(item)
    ///     }
    public func iter() -> SetIterator[T, H] {
        SetIterator(dictIter: self.dict.iter())
    }

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Inserts an element into the set.
    ///
    /// Returns true if the element was newly inserted, false if it already existed.
    ///
    /// Example:
    ///     var set: Set = [1, 2]
    ///     set.insert( 3)  // true, set is {1, 2, 3}
    ///     set.insert( 2)  // false, already present
    public mutating func insert(element: T) -> Bool {
        let oldValue = self.dict.insert(element, Unit());
        oldValue.isSome() == false
    }

    /// Inserts all elements from an iterable into the set.
    ///
    /// Example:
    ///     var set: Set = [1, 2]
    ///     set.insert(contentsOf: [3, 4, 5])  // {1, 2, 3, 4, 5}
    public mutating func insert[I](contentsOf elements: I) where I: Iterable, I.Item = T {
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    /// Adds the elements of another set to this set (mutating union).
    ///
    /// Example:
    ///     var a: Set = [1, 2]
    ///     let b: Set = [2, 3]
    ///     a.formUnion( b)  // a is {1, 2, 3}
    public mutating func formUnion(other: Set[T, H]) {
        var otherIter = other.iter();
        while let .Some(elem) = otherIter.next() {
            let _ = self.insert(elem);
        }
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes an element from the set.
    ///
    /// Returns true if the element was present and removed, false otherwise.
    ///
    /// Example:
    ///     var set: Set = [1, 2, 3]
    ///     set.remove( 2)  // true, set is {1, 3}
    ///     set.remove( 5)  // false, not present
    public mutating func remove(element: T) -> Bool {
        self.dict.remove(element).isSome()
    }

    /// Removes all elements from the set.
    ///
    /// Capacity may be retained for reuse.
    ///
    /// Example:
    ///     var set: Set = [1, 2, 3]
    ///     set.clear()  // set is {}
    public mutating func clear() {
        self.dict.clear()
    }

    /// Retains only elements that satisfy the predicate.
    ///
    /// Example:
    ///     var set: Set = [1, 2, 3, 4, 5]
    ///     set.retain(where: { (x) in x % 2 == 0 })  // {2, 4}
    public mutating func retain(matching predicate: (T) -> Bool) {
        var toRemove: Array[T] = [];
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) == false {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// Removes all elements that satisfy the predicate.
    ///
    /// The inverse of `retain(where:)`.
    ///
    /// Example:
    ///     var set: Set = [1, 2, 3, 4, 5]
    ///     set.removeAll(where: { (x) in x % 2 == 0 })  // {1, 3, 5}
    public mutating func removeAll(matching predicate: (T) -> Bool) {
        var toRemove: Array[T] = [];
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// Removes elements not in the other set (mutating intersection).
    ///
    /// Example:
    ///     var a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.formIntersection(other: b)  // a is {2, 3}
    public mutating func formIntersection(other: Set[T, H]) {
        var toRemove: Array[T] = [];
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) == false {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// Removes elements that are in the other set (mutating difference).
    ///
    /// Example:
    ///     var a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.formDifference(other: b)  // a is {1}
    public mutating func formDifference(other: Set[T, H]) {
        var toRemove: Array[T] = [];
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// Replaces this set with the symmetric difference (mutating).
    ///
    /// Example:
    ///     var a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.formSymmetricDifference(other: b)  // a is {1, 4}
    public mutating func formSymmetricDifference(other: Set[T, H]) {
        var toRemove: Array[T] = [];
        var toAdd: Array[T] = [];
        
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) {
                toRemove.append(elem);
            }
        }
        
        var otherIter = other.iter();
        while let .Some(elem) = otherIter.next() {
            if self.contains( elem) == false {
                toAdd.append(elem);
            }
        }
        
        for elem in toRemove {
            let _ = self.remove( elem);
        }
        for elem in toAdd {
            let _ = self.insert(elem);
        }
    }

    // ========================================================================
    // SET OPERATIONS (NON-MUTATING)
    // ========================================================================

    /// Returns a new set containing all elements from both sets.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [3, 4, 5]
    ///     a.union(other: b)  // {1, 2, 3, 4, 5}
    public func union(other: Set[T, H]) -> Set[T, H] {
        var result = self.clone();
        result.formUnion( other);
        result
    }

    /// Returns a new set containing only elements present in both sets.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.intersection(other: b)  // {2, 3}
    public func intersection(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count;
        var result = Set[T, H](capacity: selfCount);
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) {
                let _ = result.insert( elem);
            }
        }
        result
    }

    /// Returns a new set containing elements in this set but not in the other.
    ///
    /// Also known as "subtracting" the other set.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.difference(other: b)  // {1}
    public func difference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count;
        var result = Set[T, H](capacity: selfCount);
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) == false {
                let _ = result.insert( elem);
            }
        }
        result
    }

    /// Returns a new set containing elements in either set but not both.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [2, 3, 4]
    ///     a.symmetricDifference(other: b)  // {1, 4}
    public func symmetricDifference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count;
        let otherCount = other.count;
        var result = Set[T, H](capacity: selfCount + otherCount);
        
        // Add elements in self but not other
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) == false {
                let _ = result.insert( elem);
            }
        }
        
        // Add elements in other but not self
        var otherIter = other.iter();
        while let .Some(elem) = otherIter.next() {
            if self.contains( elem) == false {
                let _ = result.insert( elem);
            }
        }
        
        result
    }

    // ========================================================================
    // SET RELATIONS
    // ========================================================================

    /// Returns true if all elements of this set are in the other set.
    ///
    /// A set is always a subset of itself.
    ///
    /// Example:
    ///     let a: Set = [1, 2]
    ///     let b: Set = [1, 2, 3]
    ///     a.isSubset(of: b)  // true
    ///     b.isSubset(of: a)  // false
    ///     a.isSubset(of: a)  // true
    public func isSubset(of other: Set[T, H]) -> Bool {
        if self.count > other.count {
            return false
        }
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) == false {
                return false
            }
        }
        true
    }

    /// Returns true if this set is a subset of the other but not equal.
    ///
    /// Example:
    ///     let a: Set = [1, 2]
    ///     let b: Set = [1, 2, 3]
    ///     a.isStrictSubset(of: b)  // true
    ///     a.isStrictSubset(of: a)  // false
    public func isStrictSubset(of other: Set[T, H]) -> Bool {
        self.isSubset(of: other) and self.count < other.count
    }

    /// Returns true if all elements of the other set are in this set.
    ///
    /// A set is always a superset of itself.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [1, 2]
    ///     a.isSuperset(of: b)  // true
    ///     b.isSuperset(of: a)  // false
    public func isSuperset(of other: Set[T, H]) -> Bool {
        other.isSubset(of: self)
    }

    /// Returns true if this set is a superset of the other but not equal.
    ///
    /// Example:
    ///     let a: Set = [1, 2, 3]
    ///     let b: Set = [1, 2]
    ///     a.isStrictSuperset(of: b)  // true
    ///     a.isStrictSuperset(of: a)  // false
    public func isStrictSuperset(of other: Set[T, H]) -> Bool {
        self.isSuperset(of: other) and self.count > other.count
    }

    /// Returns true if this set and the other share no common elements.
    ///
    /// Example:
    ///     let a: Set = [1, 2]
    ///     let b: Set = [3, 4]
    ///     let c: Set = [2, 3]
    ///     a.isDisjoint(with: b)  // true
    ///     a.isDisjoint(with: c)  // false
    public func isDisjoint(with other: Set[T, H]) -> Bool {
        // Iterate over the smaller set for efficiency
        if self.count > other.count {
            return other.isDisjoint(with: self)
        }
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if other.contains( elem) {
                return false
            }
        }
        true
    }

    // ========================================================================
    // SEARCHING AND PREDICATES
    // ========================================================================

    /// Returns true if any element satisfies the predicate.
    ///
    /// Returns false for an empty set.
    /// Short-circuits on first matching element.
    ///
    /// Example:
    ///     Set([1, 2, 3]).contains(where: { (x) in x > 2 })  // true
    ///     Set([1, 2, 3]).contains(where: { (x) in x > 5 })  // false
    public func contains(matching predicate: (T) -> Bool) -> Bool {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                return true
            }
        }
        false
    }

    /// Returns the first element matching the predicate, or None.
    ///
    /// Note: Since set order is unspecified, "first" is arbitrary.
    ///
    /// Example:
    ///     let set: Set = [1, 2, 3, 4, 5]
    ///     set.first(where: { (x) in x > 3 })  // Some(4) or Some(5)
    public func first(matching predicate: (T) -> Bool) -> T? {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                return .Some(elem)
            }
        }
        .None
    }

    /// Returns true if all elements satisfy the predicate.
    ///
    /// Returns true for an empty set (vacuous truth).
    ///
    /// Example:
    ///     Set([2, 4, 6]).all(satisfy: { (x) in x % 2 == 0 })  // true
    ///     Set([1, 2, 4]).all(satisfy: { (x) in x % 2 == 0 })  // false
    ///     Set[Int64]().all(satisfy: { (x) in false })         // true
    public func all(satisfy predicate: (T) -> Bool) -> Bool {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) == false {
                return false
            }
        }
        true
    }

    /// Returns true if any element satisfies the predicate.
    ///
    /// Alias for `contains(where:)`.
    ///
    /// Example:
    ///     Set([1, 2, 3]).any(satisfy: { (x) in x > 2 })  // true
    public func any(satisfy predicate: (T) -> Bool) -> Bool {
        self.contains(matching: predicate)
    }

    /// Returns the count of elements satisfying the predicate.
    ///
    /// Example:
    ///     Set([1, 2, 3, 4, 5]).countWhere({ (x) in x % 2 == 0 })  // 2
    public func countWhere(predicate: (T) -> Bool) -> Int64 {
        var count: Int64 = 0;
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                count = count + 1;
            }
        }
        count
    }

    // ========================================================================
    // TRANSFORMATIONS
    // ========================================================================

    /// Returns a new set with only elements satisfying the predicate.
    ///
    /// Example:
    ///     let set: Set = [1, 2, 3, 4, 5]
    ///     let evens = set.filter(where: { (x) in x % 2 == 0 })  // {2, 4}
    public func filter(matching predicate: (T) -> Bool) -> Set[T, H] {
        var result = Set[T, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                let _ = result.insert( elem);
            }
        }
        result
    }

    /// Returns a new set with elements transformed by the function.
    ///
    /// Note: The resulting set may have fewer elements if the transform
    /// produces duplicates.
    ///
    /// Example:
    ///     let set: Set = [1, 2, 3]
    ///     let doubled = set.map(transform: { (x) in x * 2 })  // {2, 4, 6}
    ///
    ///     let words: Set = ["Hello", "WORLD"]
    ///     let lower = words.map(transform: { (s) in s.lowercase() })
    ///     // may be {"hello", "world"} or just {"hello"} if collision
    public func map[U](transform: (T) -> U) -> Set[U, H] where U: Hash {
        var result = Set[U, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            let transformed = transform(elem);
            let _ = result.insert( transformed);
        }
        result
    }

    /// Returns a new set with elements transformed, removing None results.
    ///
    /// Example:
    ///     let set: Set = ["1", "two", "3"]
    ///     let nums = set.compactMap(transform: { (s) in Int64.parse(s) })  // {1, 3}
    public func compactMap[U](transform: (T) -> U?) -> Set[U, H] where U: Hash {
        var result = Set[U, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if let .Some(transformed) = transform(elem) {
                let _ = result.insert( transformed);
            }
        }
        result
    }

    /// Returns a new set with elements transformed by a function returning sets.
    ///
    /// The resulting sets are unioned together.
    ///
    /// Example:
    ///     let set: Set = [1, 2]
    ///     let expanded = set.flatMap(transform: { (x) in Set([x, x * 10]) })
    ///     // {1, 10, 2, 20}
    public func flatMap[U](transform: (T) -> Set[U, H]) -> Set[U, H] where U: Hash {
        var result = Set[U, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            let transformedSet = transform(elem);
            result.formUnion( transformedSet);
        }
        result
    }

    // ========================================================================
    // CAPACITY MANAGEMENT
    // ========================================================================

    /// Reserves capacity for at least minimumCapacity elements.
    ///
    /// Does nothing if current capacity is already sufficient.
    ///
    /// Example:
    ///     var set = Set[String]()
    ///     set.reserveCapacity( 1000)
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        if self.capacity < minimumCapacity {
            // Create new dictionary with required capacity
            var newDict = Dictionary[T, Unit, H](capacity: minimumCapacity);
            var iter = self.iter();
            while let .Some(elem) = iter.next() {
                let _ = newDict.insert(elem, Unit());
            }
            self.dict = newDict;
        }
    }

    /// Reduces capacity to match the current count.
    ///
    /// Frees excess memory. Useful after removing many elements.
    ///
    /// Example:
    ///     var set = Set[String](capacity: 1000)
    ///     set.insert( "a")
    ///     set.shrinkToFit()  // capacity reduced
    public mutating func shrinkToFit() {
        if self.capacity > self.count {
            var newDict = Dictionary[T, Unit, H](capacity: self.count);
            var iter = self.iter();
            while let .Some(elem) = iter.next() {
                let _ = newDict.insert(elem, Unit());
            }
            self.dict = newDict;
        }
    }

    // ========================================================================
    // CONVERSIONS
    // ========================================================================

    /// Returns an array containing all elements of the set.
    ///
    /// The order of elements in the array is unspecified.
    ///
    /// Example:
    ///     let set: Set = [1, 2, 3]
    ///     let arr = set.toArray()  // [1, 2, 3] in some order
    public func toArray() -> Array[T] {
        var result = Array[T]();
        result.reserveCapacity( self.count);
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            result.append(elem);
        }
        result
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    /// Private init for clone.
    private init(dict dict: Dictionary[T, Unit, H]) {
        self.dict = dict;
    }

    /// Returns the internal dictionary (for extensions).
    func getDict() -> Dictionary[T, Unit, H] { self.dict }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// Extension for sets (always equatable since elements are Hash, which implies Equatable).
extend Set[T, H]: Equatable where T: Hash, H: Hasher, H: Defaultable {

    /// Compares two sets for equality.
    ///
    /// Two sets are equal if they contain exactly the same elements.
    ///
    /// Example:
    ///     Set([1, 2, 3]).equals(other: Set([3, 2, 1]))  // true
    ///     Set([1, 2]).equals(other: Set([1, 2, 3]))     // false
    public func equals(other: Set[T, H]) -> Bool {
        if self.count != other.count {
            return false
        }
        self.isSubset(of: other)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - FORMATTABLE
// ============================================================================

/// Formattable conformance for sets with formattable elements.
///
/// Sets format as "{elem1, elem2, elem3}".
/// Empty set formats as "{}".
///
/// Example:
///     "\{Set([1, 2, 3])}"  // "{1, 2, 3}"
///     "\{Set[Int64]()"     // "{}"
extend Set[T, H]: Formattable where T: Formattable, T: Hash, H: Hasher, H: Defaultable {
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        // Implementation: build string representation
        var result = "{";
        var first = true;
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if first == false {
                result = result + ", ";
            }
            first = false;
            result = result + elem.format(options);
        }
        result = result + "}";
        result
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - CLONEABLE
// ============================================================================

/// Cloneable conformance for sets.
///
/// Due to COW semantics, cloning is O(1) until mutation.
extend Set[T, H]: Cloneable {

    /// Creates a shallow clone of the set.
    ///
    /// Due to COW semantics, this is O(1) - the actual copy is deferred
    /// until either set is mutated.
    public func clone() -> Set[T, H] {
        Set(dict: self.dict.clone())
    }
}

/// Deep clone when T is Cloneable.
extend Set[T, H] where T: Hash, T: Cloneable, H: Hasher, H: Defaultable {

    /// Creates a deep clone of the set.
    ///
    /// Unlike `clone()` which shares storage via COW, this immediately
    /// copies all elements.
    ///
    /// Example:
    ///     let a: Set = [[1, 2], [3, 4]]  // Set of arrays
    ///     let b = a.deepClone()  // fully independent copy
    public func deepClone() -> Set[T, H] {
        var result = Set[T, H](capacity: self.count);
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            let _ = result.insert( elem.clone());
        }
        result
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - COMPARABLE
// ============================================================================

/// Extension for sets with comparable elements.
extend Set[T, H] where T: Hash, T: Comparable, H: Hasher, H: Defaultable {

    /// Returns the minimum element, or None if empty.
    ///
    /// Example:
    ///     Set([3, 1, 4]).min()  // Some(1)
    ///     Set[Int64]().min()    // None
    public func min() -> T? {
        var iter = self.iter();
        if let .Some(first) = iter.next() {
            var minValue = first;
            while let .Some(elem) = iter.next() {
                if elem < minValue {
                    minValue = elem;
                }
            }
            return .Some(minValue)
        }
        .None
    }

    /// Returns the maximum element, or None if empty.
    ///
    /// Example:
    ///     Set([3, 1, 4]).max()  // Some(4)
    ///     Set[Int64]().max()    // None
    public func max() -> T? {
        var iter = self.iter();
        if let .Some(first) = iter.next() {
            var maxValue = first;
            while let .Some(elem) = iter.next() {
                if elem > maxValue {
                    maxValue = elem;
                }
            }
            return .Some(maxValue)
        }
        .None
    }

    /// Returns a sorted array of the set's elements.
    ///
    /// Example:
    ///     Set([3, 1, 4, 1, 5]).sorted()  // [1, 3, 4, 5]
    public func sorted() -> Array[T] {
        var arr = self.toArray();
        // TODO: Sort the array once Array.sort() is available
        // For now, return unsorted
        arr
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - NUMERIC
// ============================================================================

/// Extension for sets with addable elements.
extend Set[T, H] where T: Hash, T: Addable, T.Output = T, T: Defaultable, H: Hasher, H: Defaultable {

    /// Returns the sum of all elements.
    ///
    /// Example:
    ///     Set([1, 2, 3]).sum()  // 6
    ///     Set[Int64]().sum()    // 0
    public func sum() -> T {
        var total = T();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            total = total.add(elem);
        }
        total
    }
}

// ============================================================================
// DIRECT ITERABLE CONFORMANCE
// ============================================================================

// TODO: DirectIterable protocol not yet implemented
// /// DirectIterable conformance allows using iterator methods directly on sets.
// extend Set[T, H]: DirectIterable[T] where T: Hash, H: Hasher, H: Defaultable {
//     public static func collect[I](from iter: I) -> Set[T, H] where I: Iterator, I.Item = T {
//         var result = Set[T, H]();
//         var iterator = iter;
//         while let .Some(elem) = iterator.next() {
//             let _ = result.insert( elem);
//         }
//         result
//     }
// }

// ============================================================================
// EXPRESSIBLE BY ARRAY LITERAL
// ============================================================================

/// Sets can be created from array literals with a type annotation.
///
/// Example:
///     let numbers: Set = [1, 2, 3]
///     let strings: Set[String] = ["a", "b", "c"]
///     let empty: Set[Int64] = []
extend Set[T, H]: ExpressibleByArrayLiteral where T: Hash, H: Hasher, H: Defaultable {
    type Element = T

    public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }
}
