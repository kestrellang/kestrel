// Slice[T] methods - the shared read-only surface of contiguous collections.
//
// `Array`, `UnsafeSlice` and other contiguous owners conform to
// `MutableIndirection` with `Target = Slice[T]`, so every method here is
// theirs too (member access peels to the slice), and a borrowed `&Slice[T]`
// param accepts any of them.

module std.collections

import std.core.(Bool, Equatable, Comparable, Ordering, Hashable, Hasher, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, fatalError)
import std.numeric.(Int64, UInt64, RandomNumberGenerator, DefaultRandom)
import std.result.(Optional)
import std.memory.(UnsafeSlice, UnsafeSliceIterator, Pointer)
import std.iter.(Iterable)
import std.text.(Formattable, Formatter, FormatOptions, StringBuilder, String)
import std.collections.(Array, Set, Slice, SliceIndex, SliceClampable)

/// Resolves any range-like type to a half-open `Range[Int64]` given a
/// collection length. Used by `removeSubrange` and `replaceSubrange` so
/// they accept `Range`, `ClosedRange`, `RangeFrom`, `RangeUpTo`, and
/// `RangeThrough` through a single generic parameter.
public protocol SeqRange {
    func resolve(count: Int64) -> Range[Int64]
}

// ============================================================================
// SeqRange CONFORMANCES
// ============================================================================

extend Range[Int64]: SeqRange {
    public func resolve(count: Int64) -> Range[Int64] { self }
}

extend ClosedRange[Int64]: SeqRange {
    public func resolve(count: Int64) -> Range[Int64] {
        Range[Int64](self.start, self.end + 1)
    }
}

extend RangeFrom[Int64]: SeqRange {
    public func resolve(count: Int64) -> Range[Int64] {
        Range[Int64](self.start, count)
    }
}

extend RangeUpTo[Int64]: SeqRange {
    public func resolve(count: Int64) -> Range[Int64] {
        Range[Int64](0, self.end)
    }
}

extend RangeThrough[Int64]: SeqRange {
    public func resolve(count: Int64) -> Range[Int64] {
        Range[Int64](0, self.end + 1)
    }
}

// ============================================================================
// UNIFIED SUBSCRIPTS
// ============================================================================

// Raw slices write straight through the pointer.
extend UnsafeSlice[T] {
    /// The elements as a `Slice`, in place.
    public func asSlice() -> &Slice[T] {
        unsafe Slice.lend(start: self.pointer, count: self.count)
    }

    /// Like `asSlice()`, for writing through.
    public mutating func asMutableSlice() -> &mutating Slice[T] {
        unsafe Slice.lendMutating(start: self.pointer, count: self.count)
    }

    public subscript[I](index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lend(index, self.asSlice()) }
        mutating ref { I.lendMutating(index, self.asMutableSlice()) }
    }

    public subscript[I](checked index: I)? -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lendChecked(index, self.asSlice()) }
        mutating ref { I.lendMutatingChecked(index, self.asMutableSlice()) }
    }

    public unsafe subscript[I](unchecked index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { unsafe I.lendUnchecked(index, self.asSlice()) }
        mutating ref { unsafe I.lendMutatingUnchecked(index, self.asMutableSlice()) }
    }

    public subscript(clamped index: Int64)? -> T {
        ref { self.asSlice().lendClamped(index) }
        mutating ref { self.asMutableSlice().lendMutatingClamped(index) }
    }

    public subscript[R](within range: R) -> Slice[T] where R: SliceClampable[T] {
        ref { R.lendWithin(range, self.asSlice()) }
        mutating ref { R.lendMutatingWithin(range, self.asMutableSlice()) }
    }

    public subscript(wrapped index: Int64)? -> T {
        ref { self.asSlice().lendWrapped(index) }
        mutating ref { self.asMutableSlice().lendMutatingWrapped(index) }
    }
}

// ============================================================================
// EXTEND SLICE — Read-Only Methods
// ============================================================================

extend Slice[T] where T: Copyable {

    // -- Size ----------------------------------------------------------------

    /// Half-open range `0..<count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30].indices;  // 0..<3
    /// ```
    public var indices: Range[Int64] { 0..<self.asUnsafeSlice().count }

    // -- Element access ------------------------------------------------------

    /// Lends the first element in place, or `.None` for an empty collection.
    /// O(1). A binding (`let f = xs.first`) copies it out.
    ///
    /// Read-only — to remove the first element from an `Array`, use
    /// `popFirst()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].first;  // Some(1)
    /// [].first;          // None
    /// ```
    public var first: Optional[&T] {
        let s = self.asUnsafeSlice();
        if s.count == 0 { return .None; }
        .Some(unsafe s.pointer.value)
    }

    /// Lends the last element in place, or `.None` for an empty collection.
    /// O(1).
    ///
    /// Read-only — to remove the last element from an `Array`, use
    /// `popLast()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].last;  // Some(3)
    /// [].last;          // None
    /// ```
    public var last: Optional[&T] {
        let s = self.asUnsafeSlice();
        if s.count == 0 { return .None; }
        .Some(unsafe s.pointer.offset(by: s.count - 1).value)
    }

    /// A uniformly random element drawn from `rng`, or `.None` for an
    /// empty collection. O(1).
    ///
    /// Passing the same seeded generator reproduces the same picks —
    /// use the no-argument overload for OS-entropy randomness.
    ///
    /// # Examples
    ///
    /// ```
    /// var rng = Lcg64(seed: 42);
    /// [1, 2, 3].randomElement(using: rng);  // deterministic for the seed
    /// [].randomElement(using: rng);          // None
    /// ```
    public func randomElement(mutating using rng: some RandomNumberGenerator) -> T? {
        let s = self.asUnsafeSlice();
        if s.count == 0 {
            return .None
        }
        let index = Int64(truncating: rng.nextUInt64(below: UInt64(truncating: s.count)));
        .Some(unsafe s.pointer.offset(by: index).read())
    }

    /// A uniformly random element from `DefaultRandom`, or `.None` for an
    /// empty collection. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// ["red", "green", "blue"].randomElement();  // e.g. Some("green")
    /// ```
    public func randomElement() -> T? {
        var rng = DefaultRandom();
        self.randomElement(using: rng)
    }

    // -- Iteration -----------------------------------------------------------

    // -- Pointer access (FFI) ------------------------------------------------

    /// Pointer to the first element. The pointer aliases the collection's
    /// buffer; do not outlive the source or mutate through it.
    ///
    /// # Safety
    ///
    /// Reading past `count` is undefined behavior.
    public func asPointer() -> Pointer[T] { self.asUnsafeSlice().pointer }

    // -- Validation ----------------------------------------------------------

    /// `true` if `index` is in `[0, count)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30].isValid(index: 2);   // true
    /// [10, 20, 30].isValid(index: 3);   // false
    /// [10, 20, 30].isValid(index: -1);  // false
    /// ```
    public func isValid(index index: Int64) -> Bool {
        index >= 0 and index < self.asUnsafeSlice().count
    }

    // -- Slicing -------------------------------------------------------------

    // -- Searching (predicate) -----------------------------------------------

    /// Index of the first element matching `predicate`, or `None`. O(n).
    ///
    /// Short-circuits on the first match. For value-based search on
    /// `Equatable` collections, use `firstIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].firstIndex(where: { it > 3 });   // Some(3)
    /// [1, 2, 3].firstIndex(where: { it > 10 });         // None
    /// ```
    public func firstIndex(where predicate: (T) -> Bool) -> Int64? {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(unsafe myPtr.offset(by: i).read()) {
                return .Some(i)
            }
        }
        .None
    }

    /// Index of the last element matching `predicate`, or `None`. O(n).
    ///
    /// Scans from the back; short-circuits on the first match.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2, 1].lastIndex(where: { it == 2 });  // Some(3)
    /// ```
    public func lastIndex(where predicate: (T) -> Bool) -> Int64? {
        let s = self.asUnsafeSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var i = s.count - 1;
        while i >= 0 {
            if predicate(unsafe myPtr.offset(by: i).read()) {
                return .Some(i)
            }
            i = i - 1
        }
        .None
    }

    /// Lends the first element matching `predicate`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].find(where: { it > 3 });  // Some(4)
    /// ```
    public func find(where predicate: (T) -> Bool) -> Optional[&T] {
        let s = self.asUnsafeSlice();
        for i in 0..<s.count {
            if predicate(unsafe s.pointer.offset(by: i).value) {
                return .Some(unsafe s.pointer.offset(by: i).value)
            }
        }
        .None
    }

    /// Lends the last element matching `predicate`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2, 1].findLast(where: { it > 1 });  // Some(2)
    /// ```
    public func findLast(where predicate: (T) -> Bool) -> Optional[&T] {
        let s = self.asUnsafeSlice();
        var i = s.count - 1;
        while i >= 0 {
            if predicate(unsafe s.pointer.offset(by: i).value) {
                return .Some(unsafe s.pointer.offset(by: i).value)
            }
            i = i - 1
        }
        .None
    }

    // -- Predicates ----------------------------------------------------------

    /// `true` when every element satisfies `predicate`. O(n).
    ///
    /// Short-circuits on the first failure. Vacuously true for empty
    /// collections.
    ///
    /// # Examples
    ///
    /// ```
    /// [2, 4, 6].all(where: { it % 2 == 0 });  // true
    /// [2, 3, 6].all(where: { it % 2 == 0 });  // false
    /// ```
    public func all(where predicate: (T) -> Bool) -> Bool {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(unsafe myPtr.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// `true` when at least one element satisfies `predicate`. O(n).
    ///
    /// Short-circuits on the first match. Always `false` for empty
    /// collections.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].any(where: { it > 2 });  // true
    /// [1, 2, 3].any(where: { it > 5 });  // false
    /// ```
    public func any(where predicate: (T) -> Bool) -> Bool {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(unsafe myPtr.offset(by: i).read()) {
                return true
            }
        }
        false
    }

    /// Number of elements for which `predicate` is true. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].countItems(where: { it % 2 == 0 });  // 2
    /// ```
    public func countItems(where predicate: (T) -> Bool) -> Int64 {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        var result: Int64 = 0;
        for i in 0..<s.count {
            if predicate(unsafe myPtr.offset(by: i).read()) {
                result = result + 1
            }
        }
        result
    }

    // -- Views ---------------------------------------------------------------

    // -- Eager transforms ---------------------------------------------------

    /// Maps every element through `transform` into a new array. O(n).
    ///
    /// Pre-sizes the result buffer to `self.count`, so no growth steps. For
    /// the lazy version that fuses into a chain, use `iter().map { ... }`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].map { it * 2 };       // [2, 4, 6]
    /// [1, 2, 3].map { it.format() };  // ["1", "2", "3"]
    /// ```
    public func map[U](transform: (T) -> U) -> Array[U] {
        let s = self.asUnsafeSlice();
        var b = ArrayBuilder[U](capacity: s.count);
        let p = s.pointer;
        for i in 0..<s.count {
            b.append(transform(unsafe p.offset(by: i).read()))
        }
        b.build()
    }

    /// Returns a new array containing every element matching `predicate`.
    /// O(n). Result size is unknown; uses geometric growth.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4].filter(where: { it % 2 == 0 });  // [2, 4]
    /// ```
    public func filter(where predicate: (T) -> Bool) -> Array[T] {
        let s = self.asUnsafeSlice();
        var b = ArrayBuilder[T]();
        let p = s.pointer;
        for i in 0..<s.count {
            let elem = unsafe p.offset(by: i).read();
            if predicate(elem) {
                b.append(elem)
            }
        }
        b.build()
    }

    /// Maps every element through `transform`, dropping `.None` results.
    /// O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// ["1", "x", "3"].compactMap { Int64.parse(it) };  // [1, 3]
    /// ```
    public func compactMap[U](transform: (T) -> Optional[U]) -> Array[U] {
        let s = self.asUnsafeSlice();
        var b = ArrayBuilder[U]();
        let p = s.pointer;
        for i in 0..<s.count {
            if let .Some(value) = transform(unsafe p.offset(by: i).read()) {
                b.append(value)
            }
        }
        b.build()
    }

    /// Maps every element through `transform` and concatenates the results
    /// into one flat array. O(n + total_output).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].flatMap { [it, it * 10] };  // [1, 10, 2, 20, 3, 30]
    /// ```
    public func flatMap[U](transform: (T) -> Array[U]) -> Array[U] {
        let s = self.asUnsafeSlice();
        var b = ArrayBuilder[U]();
        let p = s.pointer;
        for i in 0..<s.count {
            let inner = transform(unsafe p.offset(by: i).read());
            for j in 0..<inner.count {
                b.append(unsafe inner(unchecked: j))
            }
        }
        b.build()
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Equatable
// ============================================================================

extend Slice[T] where T: Equatable, T: Copyable {
    /// Element-wise equality. O(n).
    ///
    /// Short-circuits on the first mismatch. Order matters.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].isEqual(to: [1, 2, 3]);  // true
    /// [1, 2, 3].isEqual(to: [3, 2, 1]);  // false
    /// ```
    public func isEqual(to other: Self) -> Bool {
        let a = self.asUnsafeSlice();
        let b = other.asUnsafeSlice();
        if a.count != b.count {
            return false
        }
        for i in 0..<a.count {
            if unsafe a.pointer.offset(by: i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// `true` if the collection contains `element`. O(n).
    ///
    /// Linear scan; short-circuits on the first match.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].contains(2);  // true
    /// [1, 2, 3].contains(5);  // false
    /// ```
    public func contains(element: T) -> Bool {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if unsafe myPtr.offset(by: i).read().isEqual(to: element) {
                return true
            }
        }
        false
    }

    /// Index of the first element equal to `element`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2].firstIndex(of: 2);  // Some(1)
    /// [1, 2, 3].firstIndex(of: 5);      // None
    /// ```
    public func firstIndex(of element: T) -> Int64? {
        self.firstIndex(where: { (x) in x.isEqual(to: element) })
    }

    /// Index of the last element equal to `element`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2].lastIndex(of: 2);  // Some(3)
    /// [1, 2, 3].lastIndex(of: 5);      // None
    /// ```
    public func lastIndex(of element: T) -> Int64? {
        self.lastIndex(where: { (x) in x.isEqual(to: element) })
    }

    /// `true` if the leading elements match `prefix`. O(k) where k is
    /// the prefix length. Accepts anything that lends a `Slice[T]`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].starts(with: [1, 2]);     // true
    /// [1, 2, 3].starts(with: [2, 3]);     // false
    /// [1, 2, 3].starts(with: []);          // true (vacuous)
    /// ```
    public func starts(with prefix: &Slice[T]) -> Bool {
        let a = self.asUnsafeSlice();
        let b = prefix.asUnsafeSlice();
        if b.count > a.count {
            return false
        }
        for i in 0..<b.count {
            if unsafe a.pointer.offset(by: i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// `true` if the trailing elements match `suffix`. O(k) where k is
    /// the suffix length. Accepts anything that lends a `Slice[T]`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].ends(with: [2, 3]);  // true
    /// [1, 2, 3].ends(with: [1, 2]);  // false
    /// [1, 2, 3].ends(with: []);       // true (vacuous)
    /// ```
    public func ends(with suffix: &Slice[T]) -> Bool {
        let a = self.asUnsafeSlice();
        let b = suffix.asUnsafeSlice();
        if b.count > a.count {
            return false
        }
        let offset = a.count - b.count;
        for i in 0..<b.count {
            if unsafe a.pointer.offset(by: offset + i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Comparable
// ============================================================================

extend Slice[T] where T: Comparable, T: Copyable {
    /// Lends the smallest element, or `None` if empty. O(n).
    ///
    /// Ties go to the first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].min();  // Some(1)
    /// [].min();          // None
    /// ```
    public func min() -> Optional[&T] {
        let s = self.asUnsafeSlice();
        if s.count == 0 { return .None; }
        // Track the winner's index so nothing is copied.
        var best = 0;
        for i in 1..<s.count {
            if unsafe s.pointer.offset(by: i).value.compare(unsafe s.pointer.offset(by: best).value) == Ordering.Less { best = i; }
        }
        .Some(unsafe s.pointer.offset(by: best).value)
    }

    /// Lends the largest element, or `None` if empty. O(n).
    ///
    /// Ties go to the first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].max();  // Some(4)
    /// [].max();          // None
    /// ```
    public func max() -> Optional[&T] {
        let s = self.asUnsafeSlice();
        if s.count == 0 { return .None; }
        var best = 0;
        for i in 1..<s.count {
            if unsafe s.pointer.offset(by: i).value.compare(unsafe s.pointer.offset(by: best).value) == Ordering.Greater { best = i; }
        }
        .Some(unsafe s.pointer.offset(by: best).value)
    }

    /// `true` if elements are in non-decreasing order. O(n).
    ///
    /// Equal adjacent elements are allowed. Empty and single-element
    /// collections are vacuously sorted.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].isSorted();  // true
    /// [1, 3, 2].isSorted();  // false
    /// [1, 1, 1].isSorted();  // true
    /// [].isSorted();          // true
    /// ```
    public func isSorted() -> Bool {
        let s = self.asUnsafeSlice();
        if s.count <= 1 {
            return true
        }
        let myPtr = s.pointer;
        for i in 1..<s.count {
            if unsafe myPtr.offset(by: i).value.compare(unsafe myPtr.offset(by: i - 1).value) == Ordering.Less {
                return false
            }
        }
        true
    }

    /// Binary search for `element`. Returns its index or `None`. O(log n).
    ///
    /// When duplicates exist, which index is returned is unspecified.
    ///
    /// # Safety
    ///
    /// The collection must be sorted in ascending order. Calling on
    /// unsorted data won't crash but may produce false negatives.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].binarySearch(3);  // Some(2)
    /// [1, 2, 3, 4, 5].binarySearch(6);  // None
    /// ```
    public func binarySearch(element: T) -> Int64? {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        var lo: Int64 = 0;
        var hi: Int64 = s.count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match unsafe myPtr.offset(by: mid).value.compare(element) {
                .Less => { lo = mid + 1 },
                .Greater => { hi = mid },
                .Equal => { return .Some(mid) }
            }
        }
        .None
    }

    /// Returns a new sorted array; original unchanged. O(n log n).
    ///
    /// # Examples
    ///
    /// ```
    /// let arr = [3, 1, 4, 1, 5];
    /// arr.sorted();  // [1, 1, 3, 4, 5]
    /// // arr is still [3, 1, 4, 1, 5]
    /// ```
    public func sorted() -> Array[T] {
        var copy = Array[T]();
        let s = self.asUnsafeSlice();
        copy.reserveCapacity(s.count);
        let myPtr = s.pointer;
        for i in 0..<s.count {
            copy.append(unsafe myPtr.offset(by: i).read())
        }
        copy.sort();
        copy
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Hashable
// ============================================================================

extend Slice[T] where T: Hashable, T: Copyable {
    /// Returns a new array with duplicates removed, preserving
    /// first-occurrence order. O(n) expected: seen elements go in a `Set`.
    ///
    /// To remove only adjacent repeats, use `dedup()` / `deduped()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 1, 3, 2, 4].unique();  // [1, 2, 3, 4]
    /// ```
    public func unique() -> Array[T] {
        var result = Array[T]();
        let s = self.asUnsafeSlice();
        var seen = Set[T](capacity: s.count);
        let myPtr = s.pointer;
        for i in 0..<s.count {
            let elem = unsafe myPtr.offset(by: i).read();
            // `insert` is false when an equal element was already seen.
            if seen.insert(elem) {
                result.append(elem)
            }
        }
        result
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Formattable
// ============================================================================

extend Slice[T] where T: Formattable, T: Copyable {
    /// Renders as `"[e1, e2, ...]"`. Empty collections render as `"[]"`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].format();  // "[1, 2, 3]"
    /// [].format();          // "[]"
    /// ```
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        writer.append(char: '[');
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if i > 0 {
                writer.append(", ")
            }
            unsafe myPtr.offset(by: i).read().format(into: writer, options)
        }
        writer.append(char: ']')
    }

    /// Concatenates each element's string representation, separated by
    /// `separator`.
    ///
    /// Each element is rendered with its `format()` method using default
    /// `FormatOptions`. The default `separator` is empty (raw
    /// concatenation). An empty collection produces `""`. Works on arrays
    /// and on slices of them alike. For the bracketed debug form
    /// (`"[1, 2, 3]"`), use `format()` directly.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].joined(by: ", ");          // "1, 2, 3"
    /// [1, 2, 3].joined();                   // "123"
    /// ["a", "b", "c"](1..<3).joined(by: "-");  // "b-c"
    /// ```
    public func joined(by separator: String = "") -> String {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        var b = StringBuilder();
        for i in 0..<s.count {
            if i > 0 {
                b.append(separator)
            }
            unsafe myPtr.offset(by: i).read().format(into: b)
        }
        b.build()
    }
}
