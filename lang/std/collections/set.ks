// Set[T] - hash set with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Cloneable, Hashable, Hasher, Defaultable, Addable, Comparable)
import std.text.(Formattable, FormatOptions, StringBuilder)
import std.numeric.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Dictionary, DictionaryIterator, DefaultHasher)
import std.memory.(LiteralSlice)
import std.text.(String)
import std.core.(ExpressibleByArrayLiteral)

// ============================================================================
// INTERNAL TYPES
// ============================================================================

/// Zero-size placeholder used as the value type when storing a `Set`
/// inside a `Dictionary`.
///
/// `Set[T]` is implemented on top of `Dictionary[T, Unit]` — only the
/// keys carry information, so the value slot needs a type that
/// instances can be cheaply produced and compared. `Unit` provides
/// that: every instance is equal to every other, and `clone()` returns a
/// fresh one without copying anything meaningful. Internal-only;
/// users never see this type.
///
/// # Representation
///
/// Empty struct (zero bytes after layout).
struct Unit: Equatable, Cloneable {
    /// @name Empty
    /// Constructs the unique `Unit` value. There's nothing to
    /// initialize.
    init() {}

    /// All `Unit` instances compare equal — there's only one
    /// inhabitant.
    func isEqual(to other: Unit) -> Bool { true }
    /// Returns a fresh `Unit`. Trivial since the type carries no
    /// data.
    func clone() -> Unit { Unit() }
}

// ============================================================================
// SET ITERATOR
// ============================================================================

/// Single-pass forward iterator over the elements of a `Set[T, H]`.
///
/// Returned by `Set.iter()`. Wraps the underlying
/// `DictionaryIterator[T, Unit]` and discards the (unused) value
/// half of each entry, yielding only the key. Iteration order
/// matches the underlying bucket layout and is unspecified.
///
/// # Examples
///
/// ```
/// let set: Set = [1, 2, 3];
/// for item in set { print(item); }
/// ```
///
/// # Representation
///
/// Wraps a `DictionaryIterator[T, Unit]`.
///
/// # Memory Model
///
/// Value type. Aliases the source set's bucket array; do not retain
/// across mutations of the set.
public struct SetIterator[T, H = DefaultHasher]: Iterator where T: Hashable, H: Hasher, H: Defaultable {
    /// Element type yielded by `next()` — `T`.
    type Item = T

    /// The underlying entry iterator over the backing dictionary;
    /// only `pair.0` is read.
    private var dictIter: DictionaryIterator[T, Unit]

    /// @name From Dict
    /// Wraps a `DictionaryIterator` to yield only its keys.
    ///
    /// Low-level — prefer `Set.iter()` over calling this directly.
    public init(dictIter dictIter: DictionaryIterator[T, Unit]) {
        self.dictIter = dictIter;
    }

    /// Returns the next element, or `None` when the underlying
    /// iterator is exhausted.
    ///
    /// Once exhausted, the iterator stays exhausted.
    ///
    /// # Examples
    ///
    /// ```
    /// var it = Set([1, 2]).iter();
    /// it.next();  // Some(1)  — order unspecified
    /// it.next();  // Some(2)
    /// it.next();  // None
    /// ```
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

/// An unordered hash set of unique elements, parameterized over the
/// hasher type `H` (defaults to `DefaultHasher`).
///
/// Backed by a `Dictionary[T, Unit, H]` — the dictionary's keys are
/// the set's elements, and `Unit` fills the value slot. Inherits
/// O(1) average-case lookup, insertion, and removal, plus
/// copy-on-write storage from the underlying dictionary: copying a
/// `Set` is O(1), with the deep clone deferred until either side
/// mutates. Iteration order is unspecified. For ordered or
/// associative-style storage, see `Array[T]` and `Dictionary[K, V]`.
///
/// # Examples
///
/// ```
/// var fruits: Set = ["apple", "banana", "cherry"];
/// fruits.insert("date");
/// fruits.contains("apple");   // true
/// fruits.remove("banana");
///
/// let a: Set = [1, 2, 3];
/// let b: Set = [3, 4, 5];
/// a.union(b);                  // {1, 2, 3, 4, 5}
/// a.intersection(b);           // {3}
/// a.isSubset(of: b);           // false
/// ```
///
/// # Set Literals
///
/// Sets share array-literal syntax — you tell the compiler which one
/// you want via the type annotation:
///
/// ```
/// let empty: Set[Int64] = [];
/// let numbers: Set = [1, 2, 3];
/// let strings: Set[String] = ["a", "b", "c"];
/// ```
///
/// # Hashing
///
/// Each element's hash is computed via `T: Hashable` and stored in the
/// underlying dictionary's bucket. Swap the hasher type by writing
/// `Set[T, SipHasher]` etc.; the default `DefaultHasher` is FNV-1a
/// (see `DefaultHasher` for caveats around adversarial inputs).
///
/// # Representation
///
/// One field, `dict: Dictionary[T, Unit, H]`. All set operations
/// delegate to the dictionary.
///
/// # Memory Model
///
/// Reference-counted storage with copy-on-write *value* semantics —
/// inherited from the backing `Dictionary`. Copying a `Set` is O(1)
/// and shares storage; the next mutation triggers the deep clone so
/// the change is invisible to other copies.
///
/// # Guarantees
///
/// - Elements are unique by `Hashable`/`Equatable` equality.
/// - Iteration order is **not** specified.
/// - Operations marked O(1) are amortized; the underlying dictionary
///   resizes geometrically.
public struct Set[T, H = DefaultHasher]: Iterable, Cloneable where T: Hashable, H: Hasher, H: Defaultable {
    /// `Iterable` element type — `T`.
    type Item = T
    /// Concrete iterator type returned by `iter()`.
    type TargetIterator = SetIterator[T, H]

    /// Backing dictionary. Keys are the set's elements; values are
    /// always `Unit()`.
    var dict: Dictionary[T, Unit, H]

    // ========================================================================
    // INITIALIZERS
    // ========================================================================

    /// @name Empty
    /// Creates an empty set with no allocation.
    ///
    /// The first insert allocates the smallest dictionary bucket
    /// array (currently 8 slots). For pre-sized creation, use
    /// `init(capacity:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let set = Set[String]();
    /// set.isEmpty;   // true
    /// set.capacity;  // 0
    /// ```
    public init() {
        self.dict = Dictionary();
    }

    /// @name With Capacity
    /// Creates an empty set sized to hold at least `capacity` elements
    /// without resizing.
    ///
    /// The actual allocated capacity rounds up to the next power of
    /// two (minimum 8) per the underlying dictionary policy. A
    /// non-positive `capacity` behaves like `init()`. Panics on
    /// allocation failure.
    ///
    /// # Examples
    ///
    /// ```
    /// var set = Set[String](capacity: 1000);
    /// set.capacity;  // 1024
    /// set.count;     // 0
    /// ```
    public init(capacity capacity: Int64) {
        self.dict = Dictionary(capacity: capacity);
    }

    /// @name From Iterable
    /// Creates a set by inserting every element produced by an
    /// iterable.
    ///
    /// Duplicates collapse silently (insert returns `false` for the
    /// already-present case). Capacity grows geometrically as
    /// inserts arrive — for sized sources, follow up with
    /// `shrinkToFit()` if memory matters.
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [1, 2, 2, 3, 3, 3];
    /// let set = Set(from: arr);    // {1, 2, 3}
    /// let r   = Set(from: 1..<4);  // {1, 2, 3}
    /// ```
    public init[I](from elements: I) where I: Iterable, I.Item = T {
        self.dict = Dictionary();
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    /// @name Array Literal
    /// Creates a set from an array literal slice — emitted by the
    /// compiler when you write `let s: Set = [1, 2, 3]`.
    ///
    /// Pre-allocates capacity to the literal's element count (so the
    /// build avoids resizing) and inserts each element. Duplicates
    /// collapse.
    ///
    /// # Examples
    ///
    /// ```
    /// // Triggered by the array-literal-with-Set-annotation syntax:
    /// let nums: Set = [1, 2, 3];
    /// ```
    public init(arrayLiteral elements: LiteralSlice[T]) {
        self.dict = Dictionary(capacity: elements.count);
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Number of unique elements; O(1).
    ///
    /// Forwards to the backing dictionary's `count`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).count;   // 3
    /// Set[Int64]().count;     // 0
    /// ```
    public var count: Int64 {
        get { self.dict.count }
    }

    /// Total bucket capacity in the backing dictionary — always
    /// `>= count`.
    ///
    /// Resizes (via the dictionary's 75% load policy) trigger the
    /// next insert past the threshold. Use `reserveCapacity(...)` to
    /// pre-grow and `shrinkToFit()` to release excess.
    ///
    /// # Examples
    ///
    /// ```
    /// let set = Set[String](capacity: 100);
    /// set.capacity;  // 128
    /// ```
    public var capacity: Int64 {
        get { self.dict.capacity }
    }

    /// `true` when the set has no elements; equivalent to
    /// `count == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set[Int64]().isEmpty;   // true
    /// Set([1]).isEmpty;       // false
    /// ```
    public var isEmpty: Bool {
        get { self.dict.isEmpty }
    }

    // ========================================================================
    // MEMBERSHIP
    // ========================================================================

    /// `true` if `element` is a member of the set; O(1) average.
    ///
    /// Forwards to the dictionary's key lookup. For predicate-based
    /// search use `contains { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2, 3];
    /// set.contains(2);  // true
    /// set.contains(5);  // false
    /// ```
    public func contains(element: T) -> Bool {
        self.dict.contains(element)
    }

    /// Returns a single-pass `SetIterator[T, H]` over the elements.
    ///
    /// Order is unspecified and may change between mutations. The
    /// iterator borrows the underlying buffer; do not mutate the
    /// set while iterating.
    ///
    /// # Examples
    ///
    /// ```
    /// for item in set.iter() { print(item); }
    /// let arr = Array(from: set.iter());
    /// ```
    public func iter() -> SetIterator[T, H] {
        SetIterator(dictIter: self.dict.iter())
    }

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Inserts `element`, returning whether it was newly added.
    ///
    /// Returns `true` if the element was added, `false` if it was
    /// already present (in which case the set is unchanged). May
    /// trigger a dictionary resize and COW. For bulk inserts, see
    /// `insert(contentsOf:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2];
    /// set.insert(3);  // true; set == {1, 2, 3}
    /// set.insert(2);  // false; already present
    /// ```
    public mutating func insert(element: T) -> Bool {
        let oldValue = self.dict.insert(element, Unit());
        oldValue.isNone()
    }

    // TODO: public mutating func update(element: T) -> Optional[T]
    //
    // Swift-style replacement insert: returns the previously stored equal
    // element (or .None if newly inserted). Useful when T's equality is
    // custom (e.g. interning, case-insensitive strings) and the caller
    // wants the prior representative back.
    //
    // Blocked on a Dictionary primitive: the current
    // `Dictionary.insert(key:, value:)` overwrites the bucket with the
    // *new* key and only returns the prior `V`, dropping the prior `K`.
    // Add `Dictionary.insertReplacingEntry(key:, value:) -> Optional[(K, V)]`
    // (or similar) first, then this method becomes a one-liner.

    /// Inserts every element produced by an iterable; duplicates
    /// collapse silently.
    ///
    /// Sugar for "insert in a loop". For union with another `Set`,
    /// prefer `formUnion(...)` — it's the same semantically but
    /// reads more naturally.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2];
    /// set.insert(contentsOf: [3, 4, 5]);  // {1, 2, 3, 4, 5}
    /// set.insert(contentsOf: 5..<8);      // {1, 2, 3, 4, 5, 6, 7}
    /// ```
    public mutating func insert[I](contentsOf elements: I) where I: Iterable, I.Item = T {
        var iter = elements.iter();
        while let .Some(elem) = iter.next() {
            let _ = self.insert(elem);
        }
    }

    /// In-place union: adds every element of `other` to `self`.
    ///
    /// Mutating mirror of `union(...)`. For multi-source unions,
    /// chain calls or use `insert(contentsOf:)` over the elements.
    ///
    /// # Examples
    ///
    /// ```
    /// var a: Set = [1, 2];
    /// let b: Set = [2, 3];
    /// a.formUnion(b);  // a == {1, 2, 3}
    /// ```
    public mutating func formUnion(other: Set[T, H]) {
        var otherIter = other.iter();
        while let .Some(elem) = otherIter.next() {
            let _ = self.insert(elem);
        }
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes `element` if present; returns whether anything was
    /// removed.
    ///
    /// Leaves a tombstone in the backing dictionary — see
    /// `Dictionary.remove`. Tombstones are reclaimed by the next
    /// resize. Triggers COW only when an element is actually removed.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2, 3];
    /// set.remove(2);  // true; set == {1, 3}
    /// set.remove(5);  // false; set unchanged
    /// ```
    public mutating func remove(element: T) -> Bool {
        self.dict.remove(element).isSome()
    }

    /// Removes every element, leaving capacity untouched.
    ///
    /// Forwards to the dictionary's `clear()`. Follow with
    /// `shrinkToFit()` to release the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2, 3];
    /// set.clear();      // set == {}
    /// set.capacity;     // unchanged
    /// ```
    public mutating func clear() {
        self.dict.clear()
    }

    /// Keeps only elements for which `predicate` is true.
    ///
    /// Two-pass implementation: collects elements to remove, then
    /// deletes each. Stable in iteration semantics (set is unordered
    /// anyway). Mirror is `removeAll { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2, 3, 4, 5];
    /// set.retain { (x) in x % 2 == 0 };  // {2, 4}
    /// ```
    public mutating func retain(matching predicate: (T) -> Bool) {
        var toRemove: Array[T] = [];
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if not predicate(elem) {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// Removes every element for which `predicate` is true.
    ///
    /// Inverse of `retain { ... }`. Same two-pass structure.
    ///
    /// # Examples
    ///
    /// ```
    /// var set: Set = [1, 2, 3, 4, 5];
    /// set.removeAll { (x) in x % 2 == 0 };  // {1, 3, 5}
    /// ```
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

    /// In-place intersection: removes every element of `self` that
    /// is **not** in `other`.
    ///
    /// Mutating mirror of `intersection(...)`. Iterates over
    /// `self`, so the cost scales with `self.count`, not
    /// `other.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// var a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.formIntersection(b);  // a == {2, 3}
    /// ```
    public mutating func formIntersection(other: Set[T, H]) {
        var toRemove: Array[T] = [];
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if not other.contains( elem) {
                toRemove.append(elem);
            }
        }
        for elem in toRemove {
            let _ = self.remove( elem);
        }
    }

    /// In-place difference: removes every element of `self` that **is**
    /// in `other`.
    ///
    /// Mutating mirror of `difference(...)`. The result is "self
    /// minus other".
    ///
    /// # Examples
    ///
    /// ```
    /// var a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.formDifference(b);  // a == {1}
    /// ```
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

    /// In-place symmetric difference: keeps elements in exactly one
    /// of `self` or `other`.
    ///
    /// Mutating mirror of `symmetricDifference(...)`. Two passes:
    /// removes shared elements, then inserts elements unique to
    /// `other`.
    ///
    /// # Examples
    ///
    /// ```
    /// var a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.formSymmetricDifference(b);  // a == {1, 4}
    /// ```
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
            if not self.contains( elem) {
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

    /// Returns a new set containing every element from `self` and
    /// `other`.
    ///
    /// Non-mutating mirror of `formUnion(...)`. Internally clones
    /// `self` (cheap COW) and adds `other` into the copy.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [3, 4, 5];
    /// a.union(b);  // {1, 2, 3, 4, 5}
    /// ```
    public func union(other: Set[T, H]) -> Set[T, H] {
        var result = self.clone();
        result.formUnion( other);
        result
    }

    /// Returns a new set containing only elements present in both
    /// `self` and `other`.
    ///
    /// Non-mutating mirror of `formIntersection(...)`. For
    /// efficiency, iterates over `self`; pass the smaller set as the
    /// receiver if it matters.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.intersection(b);  // {2, 3}
    /// ```
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

    /// Returns a new set of every element in `self` that is **not**
    /// in `other` — the set difference, "self minus other".
    ///
    /// Non-mutating mirror of `formDifference(...)`. Order of
    /// arguments matters: `a.difference(b)` is generally not equal
    /// to `b.difference(a)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.difference(b);  // {1}
    /// b.difference(a);  // {4}
    /// ```
    public func difference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count;
        var result = Set[T, H](capacity: selfCount);
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if not other.contains( elem) {
                let _ = result.insert( elem);
            }
        }
        result
    }

    /// Returns a new set of elements in exactly one of `self` or
    /// `other`.
    ///
    /// Non-mutating mirror of `formSymmetricDifference(...)`.
    /// Equivalent to `union(...) - intersection(...)`. The
    /// operation is commutative — order of arguments doesn't change
    /// the result.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [2, 3, 4];
    /// a.symmetricDifference(b);  // {1, 4}
    /// ```
    public func symmetricDifference(other: Set[T, H]) -> Set[T, H] {
        let selfCount = self.count;
        let otherCount = other.count;
        var result = Set[T, H](capacity: selfCount + otherCount);
        
        // Add elements in self but not other
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if not other.contains( elem) {
                let _ = result.insert( elem);
            }
        }
        
        // Add elements in other but not self
        var otherIter = other.iter();
        while let .Some(elem) = otherIter.next() {
            if not self.contains( elem) {
                let _ = result.insert( elem);
            }
        }
        
        result
    }

    // ========================================================================
    // SET RELATIONS
    // ========================================================================

    /// `true` if every element of `self` appears in `other`.
    ///
    /// A set is always a subset of itself (reflexive). Short-circuits
    /// on the first missing element, and skips the inner scan when
    /// `self.count > other.count`. For "subset but not equal" use
    /// `isStrictSubset(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2];
    /// let b: Set = [1, 2, 3];
    /// a.isSubset(of: b);  // true
    /// b.isSubset(of: a);  // false
    /// a.isSubset(of: a);  // true
    /// ```
    public func isSubset(of other: Set[T, H]) -> Bool {
        if self.count > other.count {
            return false
        }
        var selfIter = self.iter();
        while let .Some(elem) = selfIter.next() {
            if not other.contains( elem) {
                return false
            }
        }
        true
    }

    /// `true` if `self` is a subset of `other` and the two sets are
    /// not equal.
    ///
    /// Strict (proper) subset — excludes the case where the sets are
    /// equal. Mirror of `isStrictSuperset(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2];
    /// let b: Set = [1, 2, 3];
    /// a.isStrictSubset(of: b);  // true
    /// a.isStrictSubset(of: a);  // false (equal, not strict)
    /// ```
    public func isStrictSubset(of other: Set[T, H]) -> Bool {
        self.isSubset(of: other) and self.count < other.count
    }

    /// `true` if every element of `other` appears in `self`.
    ///
    /// Reflexive (a set is its own superset). Implemented as
    /// `other.isSubset(of: self)` for code reuse.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [1, 2];
    /// a.isSuperset(of: b);  // true
    /// b.isSuperset(of: a);  // false
    /// ```
    public func isSuperset(of other: Set[T, H]) -> Bool {
        other.isSubset(of: self)
    }

    /// `true` if `self` is a superset of `other` and the two sets
    /// are not equal.
    ///
    /// Strict (proper) superset. Mirror of `isStrictSubset(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// let b: Set = [1, 2];
    /// a.isStrictSuperset(of: b);  // true
    /// a.isStrictSuperset(of: a);  // false (equal, not strict)
    /// ```
    public func isStrictSuperset(of other: Set[T, H]) -> Bool {
        self.isSuperset(of: other) and self.count > other.count
    }

    /// `true` if `self` and `other` share no elements.
    ///
    /// Iterates over the smaller set for efficiency (swaps the
    /// arguments internally if needed). Empty sets are disjoint
    /// from anything, including each other.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2];
    /// let b: Set = [3, 4];
    /// let c: Set = [2, 3];
    /// a.isDisjoint(with: b);  // true
    /// a.isDisjoint(with: c);  // false (share 2)
    /// ```
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

    /// `true` if any element satisfies `predicate`.
    ///
    /// Linear scan; short-circuits on the first match. `false` for
    /// empty sets. The aliased shape `any { ... }` exists for
    /// symmetry with `Array`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).contains { (x) in x > 2 };  // true
    /// Set([1, 2, 3]).contains { (x) in x > 5 };  // false
    /// ```
    public func contains(matching predicate: (T) -> Bool) -> Bool {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                return true
            }
        }
        false
    }

    /// Returns *some* element matching `predicate`, or `None`.
    ///
    /// "First" is determined by iteration order, which is
    /// unspecified — treat the result as arbitrary among matching
    /// elements. Short-circuits on the first match.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2, 3, 4, 5];
    /// set.first { (x) in x > 3 };   // Some(4) or Some(5)
    /// set.first { (x) in x > 99 };  // None
    /// ```
    public func first(matching predicate: (T) -> Bool) -> T? {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if predicate(elem) {
                return .Some(elem)
            }
        }
        .None
    }

    /// `true` when every element satisfies `predicate` (vacuously
    /// true for empty sets).
    ///
    /// Short-circuits on the first failure. Dual of
    /// `any { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([2, 4, 6]).all { (x) in x % 2 == 0 };  // true
    /// Set([1, 2, 4]).all { (x) in x % 2 == 0 };  // false
    /// Set[Int64]().all { (x) in false };           // true (vacuous)
    /// ```
    public func all(matching predicate: (T) -> Bool) -> Bool {
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if not predicate(elem) {
                return false
            }
        }
        true
    }

    /// `true` when at least one element satisfies `predicate`.
    ///
    /// Alias for `contains { ... }` — both names exist so
    /// predicate-style code reads naturally regardless of context.
    /// Short-circuits.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).any { (x) in x > 2 };  // true
    /// Set[Int64]().any { (x) in true };     // false (empty)
    /// ```
    public func any(matching predicate: (T) -> Bool) -> Bool {
        self.contains(matching: predicate)
    }

    /// Returns the number of elements for which `predicate` is true.
    ///
    /// Linear scan, no short-circuit. For just a presence check use
    /// `any { ... }`; for a yes/no on every element,
    /// `all { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3, 4, 5]).countItems { (x) in x % 2 == 0 };  // 2
    /// Set[Int64]().countItems { (x) in true };                // 0
    /// ```
    public func countItems(matching predicate: (T) -> Bool) -> Int64 {
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

    /// Returns a new set containing only elements for which
    /// `predicate` is true.
    ///
    /// Non-mutating mirror of `retain { ... }`. Allocates a fresh
    /// set; for in-place filtering use `retain` or
    /// `removeAll { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2, 3, 4, 5];
    /// let evens = set.filter { (x) in x % 2 == 0 };  // {2, 4}
    /// ```
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

    /// Returns a new set with each element run through `transform`.
    ///
    /// **Cardinality may shrink**: if `transform` maps two distinct
    /// elements to the same output, the result holds only one copy
    /// (sets are unique). For an `Optional`-aware variant that drops
    /// `None`, use `compactMap(...)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2, 3];
    /// let doubled = set.map { (x) in x * 2 };
    /// // {2, 4, 6}
    ///
    /// let words: Set = ["Hello", "WORLD"];
    /// let lower = words.map { (s) in s.lowercase() };
    /// // {"hello", "world"} — even though both originals lowercase to distinct strings
    /// ```
    public func map[U](transform: (T) -> U) -> Set[U, H] where U: Hashable {
        var result = Set[U, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            let transformed = transform(elem);
            let _ = result.insert( transformed);
        }
        result
    }

    /// Returns a new set with each element run through `transform`,
    /// dropping any `None` results.
    ///
    /// Useful for parse-or-skip patterns. Same uniqueness caveat as
    /// `map(...)` — collisions in the transformed values
    /// collapse.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = ["1", "two", "3"];
    /// let nums = set.compactMap { (s) in Int64.parse(s) };
    /// // {1, 3}  — "two" failed to parse
    /// ```
    public func compactMap[U](transform: (T) -> U?) -> Set[U, H] where U: Hashable {
        var result = Set[U, H]();
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if let .Some(transformed) = transform(elem) {
                let _ = result.insert( transformed);
            }
        }
        result
    }

    /// Returns a new set formed by unioning every set produced by
    /// `transform`.
    ///
    /// Each element maps to a `Set[U, H]`; those sets are merged
    /// together. The result holds the unique union — duplicates
    /// across sub-sets collapse, as with all set operations.
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2];
    /// let expanded = set.flatMap { (x) in Set([x, x * 10]) };
    /// // {1, 10, 2, 20}
    /// ```
    public func flatMap[U](transform: (T) -> Set[U, H]) -> Set[U, H] where U: Hashable {
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

    /// Grows the backing dictionary so at least `minimumCapacity`
    /// elements fit without resizing.
    ///
    /// No-op when current capacity already suffices. Implemented by
    /// rebuilding the underlying dictionary at the new capacity (a
    /// little heavier than `Dictionary.reserveCapacity` directly,
    /// since it reinserts each element). Opposite of `shrinkToFit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// var set = Set[String]();
    /// set.reserveCapacity(1000);
    /// // No reallocations for the first ~750 inserts.
    /// ```
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

    /// Reduces backing-dictionary capacity to fit the current count.
    ///
    /// Rebuilds the dictionary at a smaller capacity, dropping any
    /// tombstones. No-op when capacity already matches. Useful after
    /// large removals.
    ///
    /// # Examples
    ///
    /// ```
    /// var set = Set[String](capacity: 1000);
    /// set.insert("a");
    /// set.shrinkToFit();  // capacity drops toward count
    /// ```
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

    /// Returns an `Array[T]` with every element of the set.
    ///
    /// Order matches iteration order (i.e. unspecified). Capacity is
    /// pre-reserved to `count` so the build avoids reallocations.
    /// For an ordering, follow with `Array.sort()` or
    /// `sorted()` (in the `T: Comparable` extension below).
    ///
    /// # Examples
    ///
    /// ```
    /// let set: Set = [1, 2, 3];
    /// let arr = set.toArray();  // [1, 2, 3] in some order
    /// ```
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

    /// @name From Dict
    /// Wraps an existing backing `Dictionary` in a new `Set`. Used by
    /// `clone()` and other helpers.
    private init(dict dict: Dictionary[T, Unit, H]) {
        self.dict = dict;
    }

    /// Returns the backing `Dictionary[T, Unit, H]`. Internal helper
    /// for extensions that need direct dictionary access.
    func getDict() -> Dictionary[T, Unit, H] { self.dict }

    /// Returns a `Set` sharing the same storage; the deep copy is
    /// deferred until either side mutates.
    ///
    /// O(1) — bumps the backing dictionary's `RcBox` refcount. The
    /// first mutation on either side triggers the deep clone. For
    /// an immediate eager copy, use `deepClone()` (in the
    /// `T: Cloneable` extension below).
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [1, 2, 3];
    /// var b = a.clone();   // O(1), shares storage
    /// b.insert(4);         // b deep-copies here; a is unchanged
    /// ```
    public func clone() -> Set[T, H] {
        Set(dict: self.dict.clone())
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - EQUATABLE
// ============================================================================

/// `Equatable` conformance — every `Set[T, H]` is equatable because
/// `T: Hashable` already implies `T: Equatable`.
extend Set[T, H]: Equatable where T: Hashable, H: Hasher, H: Defaultable {

    /// `true` when `self` and `other` contain exactly the same
    /// elements.
    ///
    /// Order-independent (sets are unordered). Implemented as
    /// "equal counts and `self.isSubset(of: other)`" — short-circuits
    /// at the count check.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).isEqual(to: Set([3, 2, 1]));  // true
    /// Set([1, 2]).isEqual(to: Set([1, 2, 3]));     // false
    /// ```
    public func isEqual(to other: Set[T, H]) -> Bool {
        if self.count != other.count {
            return false
        }
        self.isSubset(of: other)
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - FORMATTABLE
// ============================================================================

/// `Formattable` conformance — renders a set as `"{e1, e2, e3}"` when
/// its elements are themselves `Formattable`.
///
/// Drives string interpolation. Empty sets render as `"{}"`. Element
/// order in the output matches iteration order and is unspecified.
extend Set[T, H]: Formattable where T: Formattable, T: Hashable, H: Hasher, H: Defaultable {
    /// Renders the set as `"{" + elements.joined(", ") + "}"`,
    /// passing `options` to each element's `format`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).format();  // "{1, 2, 3}" — order unspecified
    /// Set[Int64]().format();    // "{}"
    /// "\{Set([1, 2, 3])}";      // "{1, 2, 3}" via interpolation
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.appendChar('{');
        var first = true;
        var iter = self.iter();
        while let .Some(elem) = iter.next() {
            if not first {
                writer.append(", ")
            }
            first = false;
            elem.format(into: writer, options)
        }
        writer.appendChar('}')
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - CLONEABLE
// ============================================================================

/// Eager-copy variant of `clone()` for callers that don't want to
/// inherit the COW share with the source. Available when `T` itself
/// is `Cloneable`.
extend Set[T, H] where T: Hashable, T: Cloneable, H: Hasher, H: Defaultable {

    /// Returns a fully-detached copy of the set with no shared
    /// storage; every element is also `clone()`-d.
    ///
    /// Use over `clone()` when you specifically want to break the
    /// lazy COW share — for example, before passing the copy to
    /// another thread or system that might race with further
    /// mutations.
    ///
    /// # Examples
    ///
    /// ```
    /// let a: Set = [[1, 2], [3, 4]];  // Set of arrays
    /// let b = a.deepClone();          // fully independent copy
    /// ```
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

/// Ordering-aware operations available when `T: Comparable`.
extend Set[T, H] where T: Hashable, T: Comparable, H: Hasher, H: Defaultable {

    /// Returns the smallest element, or `None` for an empty set.
    ///
    /// Single linear pass; ties go to the first occurrence in
    /// iteration order (which is unspecified, so equally-minimal
    /// elements compare equal anyway).
    ///
    /// # Examples
    ///
    /// ```
    /// Set([3, 1, 4]).min();  // Some(1)
    /// Set[Int64]().min();    // None
    /// ```
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

    /// Returns the largest element, or `None` for an empty set.
    ///
    /// Single linear pass. Mirror of `min()`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([3, 1, 4]).max();  // Some(4)
    /// Set[Int64]().max();    // None
    /// ```
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

    /// Returns the set's elements as an ascending-sorted `Array[T]`.
    ///
    /// Convenience for "I want this set as an ordered list". Duplicates
    /// have already collapsed in the set, so the result has no repeats.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([3, 1, 4, 1, 5]).sorted();  // [1, 3, 4, 5]
    /// ```
    public func sorted() -> Array[T] {
        var arr = self.toArray();
        arr.sort();
        arr
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS - NUMERIC
// ============================================================================

/// Aggregation available when `T` forms an `Addable` monoid (`T + T = T`
/// with a `Defaultable` zero).
extend Set[T, H] where T: Hashable, T: Addable, T.Output = T, T: Defaultable, H: Hasher, H: Defaultable {

    /// Returns the sum of every element, starting from `T()` (the
    /// default-constructed zero).
    ///
    /// Empty sets return `T()` — `0` for `Int64`, `""` for `String`,
    /// etc. Linear in `count`.
    ///
    /// # Examples
    ///
    /// ```
    /// Set([1, 2, 3]).sum();  // 6
    /// Set[Int64]().sum();    // 0
    /// ```
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
// extend Set[T, H]: DirectIterable[T] where T: Hashable, H: Hasher, H: Defaultable {
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

/// `ExpressibleByArrayLiteral` conformance — what makes
/// `let s: Set = [1, 2, 3]` compile. The array-literal syntax is
/// shared with `Array`; the set form is selected by the type
/// annotation.
///
/// # Examples
///
/// ```
/// let numbers: Set = [1, 2, 3];
/// let strings: Set[String] = ["a", "b", "c"];
/// let empty: Set[Int64] = [];
/// ```
extend Set[T, H]: ExpressibleByArrayLiteral where T: Hashable, H: Hasher, H: Defaultable {
    /// `ExpressibleByArrayLiteral` element type — equals `T`.
    type Element = T

    /// @name Literal Bridge
    /// Compiler-emitted bridge for `[a, b, c]` literals constructing
    /// a `Set`.
    ///
    /// Forwards to `init(arrayLiteral:)` after wrapping the raw
    /// `(ptr, count)` in a `LiteralSlice`. Not called by user code.
    ///
    /// # Safety
    ///
    /// The compiler guarantees `_arrayLiteralPointer` covers exactly
    /// `_arrayLiteralCount` initialized elements of `T`.
    public init(_arrayLiteralPointer _arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }
}
