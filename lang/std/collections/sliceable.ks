// Sliceable[T] - shared read-only protocol for contiguous collections

module std.collections

import std.core.(Bool, Equatable, Comparable, Hashable, Hasher, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, fatalError)
import std.numeric.(Int64, UInt64, RandomNumberGenerator, SystemRandom)
import std.result.(Optional)
import std.memory.(UnsafeSlice, UnsafeSliceIterator, Pointer)
import std.iter.(Iterable)
import std.text.(Formattable, Formatter, FormatOptions, StringBuilder, String)
import std.collections.(Array)

// ============================================================================
// SLICE PROTOCOL
// ============================================================================

/// Shared read-only protocol for contiguous collections.
///
/// `Sliceable[T]` is the contiguous-collection counterpart to `Str` in
/// `std.text`: one kernel method (`asSlice`), all read-only logic in a
/// protocol extension. Both `Array[T]` and `UnsafeSlice[T]` conform, so
/// generic code constrained to `S: Sliceable[T]` accepts either without
/// overloading.
///
/// # Examples
///
/// ```
/// func sum[S](s: &S) -> Int64 where S: Sliceable[Int64], S: not Static, S: not Copyable, S: not Sized {
///     var total: Int64 = 0;
///     for elem in s { total = total + elem }
///     total
/// }
/// sum([1, 2, 3]);              // works with Array
/// sum([1, 2, 3].asUnsafeSlice());    // works with UnsafeSlice
/// ```
public protocol Sliceable[T]: Iterable {
    /// The elements as a raw `(pointer, count)` view. Only valid until the
    /// collection next changes, so shared algorithms use it within one
    /// call and never hand it out.
    func asUnsafeSlice() -> UnsafeSlice[T]
}

// ============================================================================
// UNIFIED INDEX PROTOCOLS
// ============================================================================

public protocol SeqIndex[T] {
    type SeqOutput
    func readSeq(from slice: UnsafeSlice[T]) -> SeqOutput
    func readSeqChecked(from slice: UnsafeSlice[T]) -> SeqOutput?
    func readSeqUnchecked(from slice: UnsafeSlice[T]) -> SeqOutput
    func writeSeq(to slice: UnsafeSlice[T], with value: SeqOutput)
    func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: SeqOutput)
}

public protocol SeqClampable[T] {
    type SeqClampedOutput
    func readSeqClamped(from slice: UnsafeSlice[T]) -> SeqClampedOutput
    func writeSeqClamped(to slice: UnsafeSlice[T], with value: SeqClampedOutput)
}

public protocol SeqWrappable[T] {
    type SeqWrappedOutput
    func readSeqWrapped(from slice: UnsafeSlice[T]) -> SeqWrappedOutput
    func writeSeqWrapped(to slice: UnsafeSlice[T], with value: SeqWrappedOutput)
}

/// Resolves any range-like type to a half-open `Range[Int64]` given a
/// collection length. Used by `removeSubrange` and `replaceSubrange` so
/// they accept `Range`, `ClosedRange`, `RangeFrom`, `RangeUpTo`, and
/// `RangeThrough` through a single generic parameter.
public protocol SeqRange {
    func resolve(count: Int64) -> Range[Int64]
}

// ============================================================================
// SeqIndex CONFORMANCES
// ============================================================================

extend Int64: SeqIndex[T] {
    type SeqOutput = T

    public func readSeq(from slice: UnsafeSlice[T]) -> T {
        if self < 0 or self >= slice.count {
            fatalError("Index out of bounds")
        }
        unsafe slice.pointer.offset(by: self).read()
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> T? {
        if self >= 0 and self < slice.count {
            .Some(unsafe slice.pointer.offset(by: self).read())
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> T {
        unsafe slice.pointer.offset(by: self).read()
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: T) {
        if self < 0 or self >= slice.count {
            fatalError("Index out of bounds")
        }
        unsafe slice.pointer.offset(by: self).assign(value)
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: T) {
        unsafe slice.pointer.offset(by: self).assign(value)
    }
}

extend Range[Int64]: SeqIndex[T] {
    type SeqOutput = UnsafeSlice[T]

    public func readSeq(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > slice.count or start > end {
            fatalError("Range out of bounds")
        }
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: end - start)
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T]? {
        let start = self.start;
        let end = self.end;
        if start >= 0 and end <= slice.count and start <= end {
            .Some(unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: end - start))
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: self.start), count: self.end - self.start)
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let end = self.end;
        if start < 0 or end > slice.count or start > end {
            fatalError("Range out of bounds")
        }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let rangeLen = self.end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

extend ClosedRange[Int64]: SeqIndex[T] {
    type SeqOutput = UnsafeSlice[T]

    public func readSeq(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > slice.count or start > endExclusive {
            fatalError("Range out of bounds")
        }
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start)
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T]? {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start >= 0 and endExclusive <= slice.count and start <= endExclusive {
            .Some(unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start))
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let start = self.start;
        let endExclusive = self.end + 1;
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: endExclusive - start)
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let endExclusive = self.end + 1;
        if start < 0 or endExclusive > slice.count or start > endExclusive {
            fatalError("Range out of bounds")
        }
        let rangeLen = endExclusive - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let rangeLen = self.end + 1 - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// Partial range: start.. — slice from start to end of collection
extend RangeFrom[Int64]: SeqIndex[T] {
    type SeqOutput = UnsafeSlice[T]

    public func readSeq(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let start = self.start;
        let len = slice.count;
        if start < 0 or start > len {
            fatalError("Index out of bounds")
        }
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: len - start)
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T]? {
        let start = self.start;
        let len = slice.count;
        if start >= 0 and start <= len {
            .Some(unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: len - start))
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let start = self.start;
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: slice.count - start)
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let len = slice.count;
        if start < 0 or start > len {
            fatalError("Index out of bounds")
        }
        let rangeLen = len - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let start = self.start;
        let rangeLen = slice.count - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// Partial range: ..<end — slice from beginning up to (exclusive) end
extend RangeUpTo[Int64]: SeqIndex[T] {
    type SeqOutput = UnsafeSlice[T]

    public func readSeq(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let end = self.end;
        if end < 0 or end > slice.count {
            fatalError("Index out of bounds")
        }
        unsafe UnsafeSlice(pointer: slice.pointer, count: end)
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T]? {
        let end = self.end;
        if end >= 0 and end <= slice.count {
            .Some(unsafe UnsafeSlice(pointer: slice.pointer, count: end))
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        unsafe UnsafeSlice(pointer: slice.pointer, count: self.end)
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let end = self.end;
        if end < 0 or end > slice.count {
            fatalError("Index out of bounds")
        }
        if value.count != end {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < end {
            unsafe slice.pointer.offset(by: i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let end = self.end;
        if value.count != end {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < end {
            unsafe slice.pointer.offset(by: i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// Partial range: ..=end — slice from beginning through (inclusive) end
extend RangeThrough[Int64]: SeqIndex[T] {
    type SeqOutput = UnsafeSlice[T]

    public func readSeq(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > slice.count {
            fatalError("Index out of bounds")
        }
        unsafe UnsafeSlice(pointer: slice.pointer, count: endExclusive)
    }

    public func readSeqChecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T]? {
        let endExclusive = self.end + 1;
        if endExclusive >= 0 and endExclusive <= slice.count {
            .Some(unsafe UnsafeSlice(pointer: slice.pointer, count: endExclusive))
        } else {
            .None
        }
    }

    public func readSeqUnchecked(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        unsafe UnsafeSlice(pointer: slice.pointer, count: self.end + 1)
    }

    public func writeSeq(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let endExclusive = self.end + 1;
        if endExclusive < 0 or endExclusive > slice.count {
            fatalError("Index out of bounds")
        }
        if value.count != endExclusive {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < endExclusive {
            unsafe slice.pointer.offset(by: i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }

    public func writeSeqUnchecked(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let endExclusive = self.end + 1;
        if value.count != endExclusive {
            fatalError("Slice length doesn't match range length")
        }
        var i = 0;
        while i < endExclusive {
            unsafe slice.pointer.offset(by: i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// ============================================================================
// SeqClampable CONFORMANCES
// ============================================================================

extend Int64: SeqClampable[T] {
    type SeqClampedOutput = T?

    public func readSeqClamped(from slice: UnsafeSlice[T]) -> T? {
        let len = slice.count;
        if len == 0 {
            return .None
        }
        var idx = self;
        if idx < 0 { idx = 0 }
        if idx >= len { idx = len - 1 }
        .Some(unsafe slice.pointer.offset(by: idx).read())
    }

    public func writeSeqClamped(to slice: UnsafeSlice[T], with value: T?) {
        if let .Some(v) = value {
            let len = slice.count;
            if len == 0 {
                return
            }
            var idx = self;
            if idx < 0 { idx = 0 }
            if idx >= len { idx = len - 1 }
            unsafe slice.pointer.offset(by: idx).assign(v)
        }
    }
}

extend Range[Int64]: SeqClampable[T] {
    type SeqClampedOutput = UnsafeSlice[T]

    public func readSeqClamped(from slice: UnsafeSlice[T]) -> UnsafeSlice[T] {
        let len = slice.count;
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        unsafe UnsafeSlice(pointer: slice.pointer.offset(by: start), count: end - start)
    }

    public func writeSeqClamped(to slice: UnsafeSlice[T], with value: UnsafeSlice[T]) {
        let len = slice.count;
        var start = self.start;
        var end = self.end;
        if start < 0 { start = 0 }
        if end > len { end = len }
        if start > end { start = end }
        let rangeLen = end - start;
        if value.count != rangeLen {
            fatalError("Slice length doesn't match clamped range length")
        }
        var i = 0;
        while i < rangeLen {
            unsafe slice.pointer.offset(by: start + i).assign(value.pointer.offset(by: i).read());
            i = i + 1;
        }
    }
}

// ============================================================================
// SeqWrappable CONFORMANCES
// ============================================================================

extend Int64: SeqWrappable[T] {
    type SeqWrappedOutput = T?

    public func readSeqWrapped(from slice: UnsafeSlice[T]) -> T? {
        let len = slice.count;
        if len == 0 {
            return .None
        }
        var idx = self % len;
        if idx < 0 { idx = idx + len }
        .Some(unsafe slice.pointer.offset(by: idx).read())
    }

    public func writeSeqWrapped(to slice: UnsafeSlice[T], with value: T?) {
        if let .Some(v) = value {
            let len = slice.count;
            if len == 0 {
                return
            }
            var idx = self % len;
            if idx < 0 { idx = idx + len }
            unsafe slice.pointer.offset(by: idx).assign(v)
        }
    }
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

// Read-only here. `Array`, `Slice` and `UnsafeSlice` declare their own
// subscripts (in-place places for `Array` and `Slice`), which take precedence.
extend Sliceable[T] {
    public subscript[I](index: I) -> I.SeqOutput where I: SeqIndex[T] {
        get { index.readSeq(from: self.asUnsafeSlice()) }
    }

    public subscript[I](checked index: I) -> I.SeqOutput? where I: SeqIndex[T] {
        get { index.readSeqChecked(from: self.asUnsafeSlice()) }
    }

    public subscript[I](unchecked index: I) -> I.SeqOutput where I: SeqIndex[T] {
        get { index.readSeqUnchecked(from: self.asUnsafeSlice()) }
    }

    public subscript[I](clamped index: I) -> I.SeqClampedOutput where I: SeqClampable[T] {
        get { index.readSeqClamped(from: self.asUnsafeSlice()) }
    }

    public subscript[I](wrapped index: I) -> I.SeqWrappedOutput where I: SeqWrappable[T] {
        get { index.readSeqWrapped(from: self.asUnsafeSlice()) }
    }
}

// Writable raw slices: writes go straight through the pointer.
extend UnsafeSlice[T] {
    public subscript[I](index: I) -> I.SeqOutput where I: SeqIndex[T] {
        get { index.readSeq(from: self) }
        set { index.writeSeq(to: self, with: newValue) }
    }

    public subscript[I](unchecked index: I) -> I.SeqOutput where I: SeqIndex[T] {
        get { index.readSeqUnchecked(from: self) }
        set { index.writeSeqUnchecked(to: self, with: newValue) }
    }

    public subscript[I](clamped index: I) -> I.SeqClampedOutput where I: SeqClampable[T] {
        get { index.readSeqClamped(from: self) }
        set { index.writeSeqClamped(to: self, with: newValue) }
    }

    public subscript[I](wrapped index: I) -> I.SeqWrappedOutput where I: SeqWrappable[T] {
        get { index.readSeqWrapped(from: self) }
        set { index.writeSeqWrapped(to: self, with: newValue) }
    }
}

// ============================================================================
// EXTEND SLICE — Read-Only Methods
// ============================================================================

extend Sliceable[T] {

    // -- Size ----------------------------------------------------------------

    /// Element count. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].count;  // 3
    /// [].count;          // 0
    /// ```
    public var count: Int64 { self.asUnsafeSlice().count }

    /// `true` when `count == 0`.
    ///
    /// # Examples
    ///
    /// ```
    /// [].isEmpty;   // true
    /// [1].isEmpty;  // false
    /// ```
    public var isEmpty: Bool { self.asUnsafeSlice().count == 0 }

    /// Half-open range `0..<count`.
    ///
    /// # Examples
    ///
    /// ```
    /// [10, 20, 30].indices;  // 0..<3
    /// ```
    public var indices: Range[Int64] { 0..<self.asUnsafeSlice().count }

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
        let s = self.asUnsafeSlice();
        if s.count > 0 {
            .Some(unsafe s.pointer.read())
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
        let s = self.asUnsafeSlice();
        if s.count > 0 {
            .Some(unsafe s.pointer.offset(by: s.count - 1).read())
        } else {
            .None
        }
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
        let index = Int64(from: rng.nextUInt64(below: UInt64(from: s.count)));
        .Some(unsafe s.pointer.offset(by: index).read())
    }

    /// A uniformly random element using OS entropy, or `.None` for an
    /// empty collection. O(1).
    ///
    /// # Examples
    ///
    /// ```
    /// ["red", "green", "blue"].randomElement();  // e.g. Some("green")
    /// ```
    public func randomElement() -> T? {
        var rng = SystemRandom();
        self.randomElement(using: rng)
    }

    // -- Iteration -----------------------------------------------------------

    /// Forward iterator over the elements.
    ///
    /// # Examples
    ///
    /// ```
    /// for item in [1, 2, 3] { ... }
    /// ```
    public func iter() -> UnsafeSliceIterator[T] {
        let s = self.asUnsafeSlice();
        UnsafeSliceIterator(ptr: s.pointer, remaining: s.count)
    }

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
    /// [10, 20, 30].isValidIndex(2);   // true
    /// [10, 20, 30].isValidIndex(3);   // false
    /// [10, 20, 30].isValidIndex(-1);  // false
    /// ```
    public func isValidIndex(index: Int64) -> Bool {
        index >= 0 and index < self.asUnsafeSlice().count
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
    /// [1, 2, 3, 4, 5].prefix(3);  // UnsafeSlice[1, 2, 3]
    /// [1, 2].prefix(0);            // empty slice
    /// ```
    public func prefix(count: Int64) -> UnsafeSlice[T] {
        let s = self.asUnsafeSlice();
        if count > s.count {
            fatalError("prefix: count exceeds length")
        }
        unsafe UnsafeSlice(pointer: s.pointer, count: count)
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
    /// [1, 2, 3, 4, 5].suffix(2);  // UnsafeSlice[4, 5]
    /// ```
    public func suffix(count: Int64) -> UnsafeSlice[T] {
        let s = self.asUnsafeSlice();
        if count > s.count {
            fatalError("suffix: count exceeds length")
        }
        unsafe UnsafeSlice(pointer: s.pointer.offset(by: s.count - count), count: count)
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
    /// [1, 2, 3, 4, 5].drop(first: 2);  // UnsafeSlice[3, 4, 5]
    /// ```
    public func drop(first count: Int64) -> UnsafeSlice[T] {
        let s = self.asUnsafeSlice();
        if count > s.count {
            fatalError("drop(first:): count exceeds length")
        }
        unsafe UnsafeSlice(pointer: s.pointer.offset(by: count), count: s.count - count)
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
    /// [1, 2, 3, 4, 5].drop(last: 2);  // UnsafeSlice[1, 2, 3]
    /// ```
    public func drop(last count: Int64) -> UnsafeSlice[T] {
        let s = self.asUnsafeSlice();
        if count > s.count {
            fatalError("drop(last:): count exceeds length")
        }
        unsafe UnsafeSlice(pointer: s.pointer, count: s.count - count)
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

    /// First element matching `predicate`, or `None`. O(n).
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3, 4, 5].first(where: { it > 3 });  // Some(4)
    /// ```
    public func first(where predicate: (T) -> Bool) -> T? {
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            let elem = unsafe myPtr.offset(by: i).read();
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
    /// [1, 2, 3, 2, 1].last(where: { it > 1 });  // Some(2)
    /// ```
    public func last(where predicate: (T) -> Bool) -> T? {
        let s = self.asUnsafeSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var i = s.count - 1;
        while i >= 0 {
            let elem = unsafe myPtr.offset(by: i).read();
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
                b.append(inner(unchecked: j))
            }
        }
        b.build()
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Equatable
// ============================================================================

extend Sliceable[T] where T: Equatable {
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
    /// the prefix length. Accepts any `Sliceable[T]` conformer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].starts(with: [1, 2]);     // true
    /// [1, 2, 3].starts(with: [2, 3]);     // false
    /// [1, 2, 3].starts(with: []);          // true (vacuous)
    /// ```
    public func starts(with prefix: &some Sliceable[T] and not Static and not Copyable and not Sized) -> Bool {
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
    /// the suffix length. Accepts any `Sliceable[T]` conformer.
    ///
    /// # Examples
    ///
    /// ```
    /// [1, 2, 3].ends(with: [2, 3]);  // true
    /// [1, 2, 3].ends(with: [1, 2]);  // false
    /// [1, 2, 3].ends(with: []);       // true (vacuous)
    /// ```
    public func ends(with suffix: &some Sliceable[T] and not Static and not Copyable and not Sized) -> Bool {
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

extend Sliceable[T] where T: Comparable {
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
        let s = self.asUnsafeSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var result = unsafe myPtr.read();
        for i in 1..<s.count {
            let elem = unsafe myPtr.offset(by: i).read();
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
        let s = self.asUnsafeSlice();
        if s.count == 0 {
            return .None
        }
        let myPtr = s.pointer;
        var result = unsafe myPtr.read();
        for i in 1..<s.count {
            let elem = unsafe myPtr.offset(by: i).read();
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
        let s = self.asUnsafeSlice();
        if s.count <= 1 {
            return true
        }
        let myPtr = s.pointer;
        for i in 1..<s.count {
            if unsafe myPtr.offset(by: i).read() < unsafe myPtr.offset(by: i - 1).read() {
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
            let midVal = unsafe myPtr.offset(by: mid).read();
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
        let s = self.asUnsafeSlice();
        copy.reserveCapacity(s.count);
        let myPtr = s.pointer;
        for i in 0..<s.count {
            copy.append(unsafe myPtr.offset(by: i).read())
        }
        copy.sort(by: { (a, b) in a < b });
        copy
    }
}

// ============================================================================
// EXTEND SLICE WHERE T: Hashable
// ============================================================================

extend Sliceable[T] where T: Hashable {
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
        let s = self.asUnsafeSlice();
        let myPtr = s.pointer;
        for i in 0..<s.count {
            let elem = unsafe myPtr.offset(by: i).read();
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

extend Sliceable[T] where T: Formattable {
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
}
