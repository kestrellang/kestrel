// Slice[T] - shared read-only protocol for contiguous collections

module std.collections

import std.core.(Bool, Equatable, Comparable, Hashable, Hasher, Range, ClosedRange, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(ArraySlice, ArraySliceIterator, Pointer)
import std.iter.(Iterable)
import std.text.(Formattable, FormatOptions, StringBuilder, String)
import std.collections.(Array)

// ============================================================================
// SLICE PROTOCOL
// ============================================================================

/// Shared read-only protocol for contiguous collections.
///
/// `Slice[T]` is the contiguous-collection counterpart to `Str` in
/// `std.text`: one kernel method (`asSlice`), all read-only logic in a
/// protocol extension. Both `Array[T]` and `ArraySlice[T]` conform, so
/// generic code constrained to `S: Slice[T]` accepts either without
/// overloading.
///
/// # Examples
///
/// ```
/// func sum[S](s: S) -> Int64 where S: Slice[Int64] {
///     var total: Int64 = 0;
///     for elem in s { total = total + elem }
///     total
/// }
/// sum([1, 2, 3]);              // works with Array
/// sum([1, 2, 3].asSlice());    // works with ArraySlice
/// ```
public protocol Slice[T]: Iterable {
    func asSlice() -> ArraySlice[T]
}

// ============================================================================
// EXTEND SLICE — Read-Only Methods
// ============================================================================

extend Slice[T] {

    // -- Size ----------------------------------------------------------------

    /// Element count. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].count;  // 3
    /// [].count;          // 0
    /// ```
    public var count: Int64 { self.asSlice().count }

    /// `true` when `count == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// [].isEmpty;   // true
    /// [1].isEmpty;  // false
    /// ```
    public var isEmpty: Bool { self.asSlice().count == 0 }

    /// Half-open range `0..<count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30].indices;  // 0..<3
    /// ```
    public var indices: Range[Int64] { 0..<self.asSlice().count }

    // -- Element access ------------------------------------------------------

    /// First element, or `.None` for an empty collection. O(1).
    ///
    /// Read-only — to remove the first element from an `Array`, use
    /// `popFirst()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].first();  // Some(1)
    /// [].first();          // None
    /// ```
    public func first() -> T? {
        let s = self.asSlice();
        if s.count > 0 {
            .Some(s.pointer.read())
        } else {
            .None
        }
    }

    /// Last element, or `.None` for an empty collection. O(1).
    ///
    /// Read-only — to remove the last element from an `Array`, use
    /// `pop()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].last();  // Some(3)
    /// [].last();          // None
    /// ```
    public func last() -> T? {
        let s = self.asSlice();
        if s.count > 0 {
            .Some(s.pointer.offset(by: s.count - 1).read())
        } else {
            .None
        }
    }

    // -- Iteration -----------------------------------------------------------

    /// Forward iterator over the elements.
    ///
    /// # Examples
    ///
    /// ```
    /// for item in [1, 2, 3] { ... }
    /// ```
    public func iter() -> ArraySliceIterator[T] {
        let s = self.asSlice();
        ArraySliceIterator(ptr: s.pointer, remaining: s.count)
    }

    // -- Pointer access (FFI) ------------------------------------------------

    /// Pointer to the first element. The pointer aliases the collection's
    /// buffer; do not outlive the source or mutate through it.
    ///
    /// # Safety
    ///
    /// Reading past `count` is undefined behavior.
    public func asPointer() -> Pointer[T] { self.asSlice().pointer }

    // -- Validation ----------------------------------------------------------

    /// `true` if `index` is in `[0, count)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30].isValidIndex(2);   // true
    /// [10, 20, 30].isValidIndex(3);   // false
    /// [10, 20, 30].isValidIndex(-1);  // false
    /// ```
    public func isValidIndex(index: Int64) -> Bool {
        index >= 0 and index < self.asSlice().count
    }

    // -- Slicing -------------------------------------------------------------

    /// Returns a slice over the first `count` elements. O(1).
    ///
    /// # Errors
    ///
    /// Panics if `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].prefix(3);  // ArraySlice[1, 2, 3]
    /// [1, 2].prefix(0);            // empty slice
    /// ```
    public func prefix(count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("prefix: count exceeds length")
        }
        ArraySlice(pointer: s.pointer, count: count)
    }

    /// Returns a slice over the last `count` elements. O(1).
    ///
    /// # Errors
    ///
    /// Panics if `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].suffix(2);  // ArraySlice[4, 5]
    /// ```
    public func suffix(count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("suffix: count exceeds length")
        }
        ArraySlice(pointer: s.pointer.offset(by: s.count - count), count: count)
    }

    /// Returns a slice with the first `count` elements skipped. O(1).
    ///
    /// Complement of `prefix`.
    ///
    /// # Errors
    ///
    /// Panics if `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].drop(first: 2);  // ArraySlice[3, 4, 5]
    /// ```
    public func drop(first count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("drop(first:): count exceeds length")
        }
        ArraySlice(pointer: s.pointer.offset(by: count), count: s.count - count)
    }

    /// Returns a slice with the last `count` elements skipped. O(1).
    ///
    /// Complement of `suffix`.
    ///
    /// # Errors
    ///
    /// Panics if `count > self.count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].drop(last: 2);  // ArraySlice[1, 2, 3]
    /// ```
    public func drop(last count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("drop(last:): count exceeds length")
        }
        ArraySlice(pointer: s.pointer, count: s.count - count)
    }

    // -- Searching (predicate) -----------------------------------------------

    /// Index of the first element matching `predicate`, or `None`. O(n).
    ///
    /// Short-circuits on the first match. For value-based search on
    /// `Equatable` collections, use `firstIndex(of:)`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].firstIndex(matching: { it > 3 });   // Some(3)
    /// [1, 2, 3].firstIndex(matching: { it > 10 });         // None
    /// ```
    public func firstIndex(matching predicate: (T) -> Bool) -> Int64? {
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(myPtr.offset(by: i).read()) {
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
    /// [1, 2, 3, 2, 1].lastIndex(matching: { it == 2 });  // Some(3)
    /// ```
    public func lastIndex(matching predicate: (T) -> Bool) -> Int64? {
        let s = self.asSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var i = s.count - 1;
        while i >= 0 {
            if predicate(myPtr.offset(by: i).read()) {
                return .Some(i)
            }
            i = i - 1
        }
        .None
    }

    /// First element matching `predicate`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].first(matching: { it > 3 });  // Some(4)
    /// ```
    public func first(matching predicate: (T) -> Bool) -> T? {
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            let elem = myPtr.offset(by: i).read();
            if predicate(elem) {
                return .Some(elem)
            }
        }
        .None
    }

    /// Last element matching `predicate`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 2, 1].last(matching: { it > 1 });  // Some(2)
    /// ```
    public func last(matching predicate: (T) -> Bool) -> T? {
        let s = self.asSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var i = s.count - 1;
        while i >= 0 {
            let elem = myPtr.offset(by: i).read();
            if predicate(elem) {
                return .Some(elem)
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
    /// [2, 4, 6].all(matching: { it % 2 == 0 });  // true
    /// [2, 3, 6].all(matching: { it % 2 == 0 });  // false
    /// ```
    public func all(matching predicate: (T) -> Bool) -> Bool {
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(myPtr.offset(by: i).read()) == false {
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
    /// [1, 2, 3].any(matching: { it > 2 });  // true
    /// [1, 2, 3].any(matching: { it > 5 });  // false
    /// ```
    public func any(matching predicate: (T) -> Bool) -> Bool {
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if predicate(myPtr.offset(by: i).read()) {
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
    /// [1, 2, 3, 4, 5].countItems(matching: { it % 2 == 0 });  // 2
    /// ```
    public func countItems(matching predicate: (T) -> Bool) -> Int64 {
        let s = self.asSlice();
        let myPtr = s.pointer;
        var result: Int64 = 0;
        for i in 0..<s.count {
            if predicate(myPtr.offset(by: i).read()) {
                result = result + 1
            }
        }
        result
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Equatable
// ============================================================================

extend Slice[T] where T: Equatable {
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
        let a = self.asSlice();
        let b = other.asSlice();
        if a.count != b.count {
            return false
        }
        for i in 0..<a.count {
            if a.pointer.offset(by: i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
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
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if myPtr.offset(by: i).read().isEqual(to: element) {
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
        self.firstIndex(matching: { (x) in x.isEqual(to: element) })
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
        self.lastIndex(matching: { (x) in x.isEqual(to: element) })
    }

    /// `true` if the leading elements match `prefix`. O(k) where k is
    /// the prefix length. Accepts any `Slice[T]` conformer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].starts(with: [1, 2]);     // true
    /// [1, 2, 3].starts(with: [2, 3]);     // false
    /// [1, 2, 3].starts(with: []);          // true (vacuous)
    /// ```
    public func starts[S](with prefix: S) -> Bool where S: Slice[T] {
        let a = self.asSlice();
        let b = prefix.asSlice();
        if b.count > a.count {
            return false
        }
        for i in 0..<b.count {
            if a.pointer.offset(by: i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// `true` if the trailing elements match `suffix`. O(k) where k is
    /// the suffix length. Accepts any `Slice[T]` conformer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].ends(with: [2, 3]);  // true
    /// [1, 2, 3].ends(with: [1, 2]);  // false
    /// [1, 2, 3].ends(with: []);       // true (vacuous)
    /// ```
    public func ends[S](with suffix: S) -> Bool where S: Slice[T] {
        let a = self.asSlice();
        let b = suffix.asSlice();
        if b.count > a.count {
            return false
        }
        let offset = a.count - b.count;
        for i in 0..<b.count {
            if a.pointer.offset(by: offset + i).read().isEqual(to: b.pointer.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// Splits on each element equal to `separator`. O(n).
    ///
    /// Separators are dropped; empty runs between adjacent separators
    /// are preserved. The result always has `(separatorCount + 1)`
    /// elements. The returned slices alias the source buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 0, 2, 0, 3].split(0);
    /// // [ArraySlice[1], ArraySlice[2], ArraySlice[3]]
    ///
    /// [1, 2, 3].split(0);
    /// // [ArraySlice[1, 2, 3]] — separator not found
    /// ```
    public func split(separator: T) -> Array[ArraySlice[T]] {
        var result = Array[ArraySlice[T]]();
        let s = self.asSlice();
        let myPtr = s.pointer;
        var start: Int64 = 0;
        for i in 0..<s.count {
            if myPtr.offset(by: i).read().isEqual(to: separator) {
                result.append(ArraySlice(pointer: myPtr.offset(by: start), count: i - start));
                start = i + 1
            }
        }
        result.append(ArraySlice(pointer: myPtr.offset(by: start), count: s.count - start));
        result
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Comparable
// ============================================================================

extend Slice[T] where T: Comparable {
    /// Smallest element, or `None` if empty. O(n).
    ///
    /// Ties go to the first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].min();  // Some(1)
    /// [].min();          // None
    /// ```
    public func min() -> T? {
        let s = self.asSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var result = myPtr.read();
        for i in 1..<s.count {
            let elem = myPtr.offset(by: i).read();
            if elem < result {
                result = elem
            }
        }
        .Some(result)
    }

    /// Largest element, or `None` if empty. O(n).
    ///
    /// Ties go to the first occurrence.
    ///
    /// # Examples
    ///
    /// ```
    /// [3, 1, 4].max();  // Some(4)
    /// [].max();          // None
    /// ```
    public func max() -> T? {
        let s = self.asSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var result = myPtr.read();
        for i in 1..<s.count {
            let elem = myPtr.offset(by: i).read();
            if elem > result {
                result = elem
            }
        }
        .Some(result)
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
        let s = self.asSlice();
        if s.count <= 1 {
            return true
        }
        let myPtr = s.pointer;
        for i in 1..<s.count {
            if myPtr.offset(by: i).read() < myPtr.offset(by: i - 1).read() {
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
        let s = self.asSlice();
        let myPtr = s.pointer;
        var lo: Int64 = 0;
        var hi: Int64 = s.count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let midVal = myPtr.offset(by: mid).read();
            if midVal < element {
                lo = mid + 1
            } else if midVal > element {
                hi = mid
            } else {
                return .Some(mid)
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
        let s = self.asSlice();
        copy.reserveCapacity(s.count);
        let myPtr = s.pointer;
        for i in 0..<s.count {
            copy.append(myPtr.offset(by: i).read())
        }
        copy.sort(by: { (a, b) in a < b });
        copy
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Hashable
// ============================================================================

extend Slice[T] where T: Hashable {
    /// Returns a new array with duplicates removed, preserving
    /// first-occurrence order. O(n²).
    ///
    /// For the mutating variant on `Array`, see `removeDuplicates()`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 1, 3, 2, 4].unique();  // [1, 2, 3, 4]
    /// ```
    public func unique() -> Array[T] {
        var result = Array[T]();
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            let elem = myPtr.offset(by: i).read();
            var found = false;
            for j in 0..<result.count {
                if result(unchecked: j).isEqual(to: elem) {
                    found = true
                }
            }
            if found == false {
                result.append(elem)
            }
        }
        result
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Formattable
// ============================================================================

extend Slice[T] where T: Formattable {
    /// Renders as `"[e1, e2, ...]"`. Empty collections render as `"[]"`.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].format();  // "[1, 2, 3]"
    /// [].format();          // "[]"
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
        writer.appendChar('[');
        let s = self.asSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            if i > 0 {
                writer.append(", ")
            }
            myPtr.offset(by: i).read().format(into: writer, options)
        }
        writer.appendChar(']')
    }
}
