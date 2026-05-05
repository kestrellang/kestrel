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
/// Requires exactly one method from conformers: `asSlice()`. All
/// read-only methods are defined once in `extend Slice` and inherited
/// by both `Array[T]` and `ArraySlice[T]` automatically.
public protocol Slice[T]: Iterable {
    func asSlice() -> ArraySlice[T]
}

// ============================================================================
// EXTEND SLICE — Read-Only Methods
// ============================================================================

extend Slice[T] {

    // -- Size ----------------------------------------------------------------

    /// Element count. O(1).
    public var count: Int64 { self.asSlice().count }

    /// `true` when `count == 0`.
    public var isEmpty: Bool { self.asSlice().count == 0 }

    /// Half-open range `0..<count`.
    public var indices: Range[Int64] { 0..<self.asSlice().count }

    // -- Element access ------------------------------------------------------

    /// First element, or `.None` for an empty collection.
    public func first() -> T? {
        let s = self.asSlice();
        if s.count > 0 {
            .Some(s.pointer.read())
        } else {
            .None
        }
    }

    /// Last element, or `.None` for an empty collection.
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
    public func iter() -> ArraySliceIterator[T] {
        let s = self.asSlice();
        ArraySliceIterator(ptr: s.pointer, remaining: s.count)
    }

    // -- Pointer access (FFI) ------------------------------------------------

    /// Pointer to the first element.
    public func asPointer() -> Pointer[T] { self.asSlice().pointer }

    // -- Validation ----------------------------------------------------------

    /// `true` if `index` is in `[0, count)`.
    public func isValidIndex(index: Int64) -> Bool {
        index >= 0 and index < self.asSlice().count
    }

    // -- Slicing -------------------------------------------------------------

    /// Returns a slice over the first `count` elements.
    public func prefix(count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("prefix: count exceeds length")
        }
        ArraySlice(pointer: s.pointer, count: count)
    }

    /// Returns a slice over the last `count` elements.
    public func suffix(count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("suffix: count exceeds length")
        }
        ArraySlice(pointer: s.pointer.offset(by: s.count - count), count: count)
    }

    /// Returns a slice with the first `count` elements skipped.
    public func drop(first count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("drop(first:): count exceeds length")
        }
        ArraySlice(pointer: s.pointer.offset(by: count), count: s.count - count)
    }

    /// Returns a slice with the last `count` elements skipped.
    public func drop(last count: Int64) -> ArraySlice[T] {
        let s = self.asSlice();
        if count > s.count {
            fatalError("drop(last:): count exceeds length")
        }
        ArraySlice(pointer: s.pointer, count: s.count - count)
    }

    // -- Searching (predicate) -----------------------------------------------

    /// Index of the first element matching `predicate`, or `None`.
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

    /// Index of the last element matching `predicate`, or `None`.
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

    /// First element matching `predicate`, or `None`.
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

    /// Last element matching `predicate`, or `None`.
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

    /// `true` when every element satisfies `predicate`.
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

    /// `true` when at least one element satisfies `predicate`.
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

    /// Number of elements for which `predicate` is true.
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
    /// Element-wise equality.
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

    /// `true` if the collection contains `element`.
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

    /// Index of the first element equal to `element`, or `None`.
    public func firstIndex(of element: T) -> Int64? {
        self.firstIndex(matching: { (x) in x.isEqual(to: element) })
    }

    /// Index of the last element equal to `element`, or `None`.
    public func lastIndex(of element: T) -> Int64? {
        self.lastIndex(matching: { (x) in x.isEqual(to: element) })
    }

    /// `true` if the leading elements match `prefix`.
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

    /// `true` if the trailing elements match `suffix`.
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

    /// Splits on each element equal to `separator`.
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
    /// Smallest element, or `None` if empty.
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

    /// Largest element, or `None` if empty.
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

    /// `true` if elements are in non-decreasing order.
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

    /// Binary search for `element`. Returns its index or `None`.
    /// The collection must be sorted in ascending order.
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

    /// Returns a new sorted array; original unchanged.
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
    /// Returns a new array with duplicates removed, preserving first-occurrence order.
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
    /// Renders as `"[e1, e2, ...]"`.
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
