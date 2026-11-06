// Slice[T] - a run of contiguous elements, viewed in place

module std.collections

import std.core.(Bool, Equatable, ArrayMatchable, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(UnsafeSlice, UnsafeSliceIterator, Pointer)
import std.iter.(Iterable)
import std.collections.(Sliceable, SeqRange)

// ============================================================================
// SLICE
// ============================================================================

/// A run of `T`s laid out back to back, viewed in place.
///
/// `Slice[T]` is unsized: you only ever hold one behind a reference,
/// `&Slice[T]` or `&mutating Slice[T]`, which is two words (the first
/// element's address and the count). `xs(1..<3)` on an `Array` or on
/// another slice lends one; nothing is copied.
///
/// A slice doesn't own its elements and doesn't keep the buffer alive. A
/// slice of an `Array` is only valid until the array next reallocates
/// (`append`, `reserveCapacity`, …) or is dropped; using it after that
/// reads freed memory. The compiler does not check this.
///
/// # Examples
///
/// ```
/// var xs = [1, 2, 3, 4, 5];
/// let mid = &xs(1..<4);   // [2, 3, 4]
/// mid(0);                 // 2
/// mid(1..<3);             // [3, 4], a slice of the same elements
/// xs(1..<4)(0) = 20;      // xs == [1, 20, 3, 4, 5]
/// ```
public struct Slice[T]: not Copyable {
    fileprivate var elements: lang.slice[T]
}

extend Slice[T] {
    /// @name From Parts
    /// Lends the `count` elements starting at `start` as a slice. The
    /// caller is responsible for `count` live elements being there for as
    /// long as the slice is used.
    public static func lend(consuming start start: Pointer[T], consuming count count: Int64) -> &Slice[T] {
        Slice.at(start, count).value
    }

    /// @name From Parts (Mutating)
    /// Like `lend(start:count:)`, for writing through.
    public static func lendMutating(consuming start start: Pointer[T], consuming count count: Int64) -> &mutating Slice[T] {
        Slice.at(start, count).mutatingValue
    }

    /// A typed pointer to the fat `(start, count)` pair.
    fileprivate static func at(start: Pointer[T], count: Int64) -> Pointer[Slice[T]] {
        let fat: lang.ptr[lang.slice[T]] = lang.slice_from_parts(start.raw, count.raw);
        Pointer(raw: lang.cast_ptr[_, Slice[T]](fat))
    }

    /// This slice's own fat pointer.
    fileprivate func fat() -> lang.ptr[lang.slice[T]] {
        lang.cast_ptr[_, lang.slice[T]](lang.ptr_to(self))
    }

    /// Element count.
    public var count: Int64 { Int64(intLiteral: lang.slice_count(self.fat())) }

    /// `true` when `count == 0`.
    public var isEmpty: Bool { self.count == 0 }

    /// Pointer to the first element.
    public var pointer: Pointer[T] { Pointer(raw: lang.cast_ptr[_, T](self.fat())) }

    /// Element access and sub-slicing, in place: `s(2)` is a reference to
    /// an element, `s(1..<3)` a narrower slice of the same elements. Panics
    /// when the index or range doesn't fit.
    public subscript[I](index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lend(index, self) }
        mutating ref { I.lendMutating(index, self) }
    }

    /// Like `self(index)`, without the bounds check (undefined behavior when
    /// out of range).
    public subscript[I](unchecked index: I) -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lendUnchecked(index, self) }
        mutating ref { I.lendMutatingUnchecked(index, self) }
    }

    /// Like `self(index)`, but the place may not exist: `.None` instead of
    /// a panic when out of range. Reading copies the element out (`T?`) or
    /// lends the slice; `if let .Some(&mutating v) = s(checked: i)` writes
    /// in place.
    public subscript[I](checked index: I)? -> I.SliceOutput where I: SliceIndex[T] {
        ref { I.lendChecked(index, self) }
        mutating ref { I.lendMutatingChecked(index, self) }
    }

    /// The element at `index` saturated into `0..<count`, in place, or
    /// `.None` for an empty slice.
    public subscript(clamped index: Int64)? -> T {
        ref { self.lendClamped(index) }
        mutating ref { self.lendMutatingClamped(index) }
    }

    /// The part of `range` that lies within the slice, in place (always
    /// valid, possibly empty).
    public subscript[R](within range: R) -> Slice[T] where R: SliceClampable[T] {
        ref { R.lendWithin(range, self) }
        mutating ref { R.lendMutatingWithin(range, self) }
    }

    /// The element at `index` modulo `count` (negative indices count from
    /// the end), in place, or `.None` for an empty slice.
    public subscript(wrapped index: Int64)? -> T {
        ref { self.lendWrapped(index) }
        mutating ref { self.lendMutatingWrapped(index) }
    }

    /// Lends the element `xs(clamped: index)` names.
    internal func lendClamped(consuming index: Int64) -> Optional[&T] {
        let i = clampSliceIndex(index, self.count);
        if i < 0 { return .None; }
        .Some(self.pointer.offset(by: i).value)
    }

    /// Like `lendClamped`, lent mutably.
    internal mutating func lendMutatingClamped(consuming index: Int64) -> Optional[&mutating T] {
        let i = clampSliceIndex(index, self.count);
        if i < 0 { return .None; }
        .Some(self.pointer.offset(by: i).mutatingValue)
    }

    /// Lends the element `xs(wrapped: index)` names.
    internal func lendWrapped(consuming index: Int64) -> Optional[&T] {
        let i = wrapSliceIndex(index, self.count);
        if i < 0 { return .None; }
        .Some(self.pointer.offset(by: i).value)
    }

    /// Like `lendWrapped`, lent mutably.
    internal mutating func lendMutatingWrapped(consuming index: Int64) -> Optional[&mutating T] {
        let i = wrapSliceIndex(index, self.count);
        if i < 0 { return .None; }
        .Some(self.pointer.offset(by: i).mutatingValue)
    }
}

extend Slice[T] where T: Copyable {
    /// Overwrites every element with the matching element of `other`.
    /// Panics if the counts differ.
    public mutating func assign(from other: &some Sliceable[T] and not Static and not Copyable and not Sized) {
        let src = other.asUnsafeSlice();
        let n = self.count;
        if src.count != n {
            fatalError("Slice.assign(from:): count mismatch");
        }
        let dst = self.pointer;
        var i: Int64 = 0;
        while i < n {
            dst.offset(by: i).mutatingValue = src.pointer.offset(by: i).read();
            i += 1;
        }
    }

    /// An owned copy of the elements.
    public func toArray() -> Array[T] {
        var out: Array[T] = [];
        out.reserveCapacity(self.count);
        for x in self.asUnsafeSlice() {
            out.append(x);
        }
        out
    }
}

/// `Sliceable[T]` conformance: every read-only algorithm on `Sliceable`
/// works on a slice too.
extend Slice[T]: Sliceable[T], Iterable {
    type Item = T
    type TargetIterator = UnsafeSliceIterator[T]

    /// Itself: a slice is already the in-place view.
    public func asSlice() -> &Slice[T] { self }

    /// The elements as a raw `(pointer, count)` view.
    public func asUnsafeSlice() -> UnsafeSlice[T] {
        UnsafeSlice(pointer: self.pointer, count: self.count)
    }

    /// Forward iterator over the elements.
    public func iter() -> UnsafeSliceIterator[T] {
        self.asUnsafeSlice().iter()
    }
}

extend Slice[T] {
    /// The first `count` elements. Panics if `count > self.count`.
    public func prefix(count: Int64) -> &Slice[T] { self(..<count) }

    /// The last `count` elements. Panics if `count > self.count`.
    public func suffix(count: Int64) -> &Slice[T] { self((self.count - count)..) }

    /// Everything after the first `count` elements.
    public func drop(first count: Int64) -> &Slice[T] { self(count..) }

    /// Everything before the last `count` elements.
    public func drop(last count: Int64) -> &Slice[T] { self(..<(self.count - count)) }
}

extend Array[T] {
    /// See `Slice.prefix(count:)`.
    public func prefix(count: Int64) -> &Slice[T] { self(..<count) }

    /// See `Slice.suffix(count:)`.
    public func suffix(count: Int64) -> &Slice[T] { self((self.count - count)..) }

    /// See `Slice.drop(first:)`.
    public func drop(first count: Int64) -> &Slice[T] { self(count..) }

    /// See `Slice.drop(last:)`.
    public func drop(last count: Int64) -> &Slice[T] { self(..<(self.count - count)) }
}

/// Element-wise equality (`isEqual` comes from `extend Sliceable[T] where
/// T: Equatable`).
extend Slice[T]: Equatable where T: Equatable { }

/// `ArrayMatchable` conformance so a slice can appear in array patterns.
/// `..rest` binds a narrower slice of the same elements.
extend Slice[T]: ArrayMatchable {
    type Element = T
    type Rest = &Slice[T]

    public func matchLength() -> Int64 { self.count }

    public func matchGet(index: Int64) -> T { self.pointer.offset(by: index).read() }

    public func matchSlice(from: Int64, to: Int64) -> &Slice[T] { self(unchecked: from..<to) }
}

// ============================================================================
// INDEX PROTOCOLS
// ============================================================================

/// An index into a `Slice[T]`: an `Int64` lends an element (`&T`), a range
/// lends a narrower `Slice[T]`.
///
/// The requirements are static so the lent reference is rooted at `base`
/// (a method's returned reference is rooted at its receiver, which would be
/// the index).
public protocol SliceIndex[T] {
    type SliceOutput
    @sugarOnly
    static func lend(consuming index: Self, base: Slice[T]) -> &SliceOutput
    @sugarOnly
    static func lendMutating(consuming index: Self, mutating base: Slice[T]) -> &mutating SliceOutput
    @sugarOnly
    static func lendUnchecked(consuming index: Self, base: Slice[T]) -> &SliceOutput
    @sugarOnly
    static func lendMutatingUnchecked(consuming index: Self, mutating base: Slice[T]) -> &mutating SliceOutput
    /// `.None` when the index or range doesn't fit.
    @sugarOnly
    static func lendChecked(consuming index: Self, base: Slice[T]) -> Optional[&SliceOutput]
    @sugarOnly
    static func lendMutatingChecked(consuming index: Self, mutating base: Slice[T]) -> Optional[&mutating SliceOutput]
}

/// A range that `xs(within: range)` clamps into the slice and lends.
public protocol SliceClampable[T] {
    @sugarOnly
    static func lendWithin(consuming range: Self, base: Slice[T]) -> &Slice[T]
    @sugarOnly
    static func lendMutatingWithin(consuming range: Self, mutating base: Slice[T]) -> &mutating Slice[T]
}

extend Int64: SliceIndex[T] {
    type SliceOutput = T

    public static func lend(consuming index: Int64, base: Slice[T]) -> &T {
        if index < 0 or index >= base.count {
            fatalError("Slice index out of bounds");
        }
        base.pointer.offset(by: index).value
    }

    public static func lendMutating(consuming index: Int64, mutating base: Slice[T]) -> &mutating T {
        if index < 0 or index >= base.count {
            fatalError("Slice index out of bounds");
        }
        base.pointer.offset(by: index).mutatingValue
    }

    public static func lendUnchecked(consuming index: Int64, base: Slice[T]) -> &T {
        base.pointer.offset(by: index).value
    }

    public static func lendMutatingUnchecked(consuming index: Int64, mutating base: Slice[T]) -> &mutating T {
        base.pointer.offset(by: index).mutatingValue
    }

    public static func lendChecked(consuming index: Int64, base: Slice[T]) -> Optional[&T] {
        if index < 0 or index >= base.count {
            return .None;
        }
        .Some(base.pointer.offset(by: index).value)
    }

    public static func lendMutatingChecked(consuming index: Int64, mutating base: Slice[T]) -> Optional[&mutating T] {
        if index < 0 or index >= base.count {
            return .None;
        }
        .Some(base.pointer.offset(by: index).mutatingValue)
    }
}

extend Range[Int64]: SliceIndex[T] {
    type SliceOutput = Slice[T]
    public static func lend(consuming index: Range[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, checkedSliceRange(index, base.count)).value }
    public static func lendMutating(consuming index: Range[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, checkedSliceRange(index, base.count)).mutatingValue }
    public static func lendUnchecked(consuming index: Range[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, index).value }
    public static func lendMutatingUnchecked(consuming index: Range[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, index).mutatingValue }
    public static func lendChecked(consuming index: Range[Int64], base: Slice[T]) -> Optional[&Slice[T]] { checkedSubslice(base, index) }
    public static func lendMutatingChecked(consuming index: Range[Int64], mutating base: Slice[T]) -> Optional[&mutating Slice[T]] { checkedSubsliceMutating(base, index) }
}

extend ClosedRange[Int64]: SliceIndex[T] {
    type SliceOutput = Slice[T]
    public static func lend(consuming index: ClosedRange[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).value }
    public static func lendMutating(consuming index: ClosedRange[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).mutatingValue }
    public static func lendUnchecked(consuming index: ClosedRange[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, index.resolve(base.count)).value }
    public static func lendMutatingUnchecked(consuming index: ClosedRange[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, index.resolve(base.count)).mutatingValue }
    public static func lendChecked(consuming index: ClosedRange[Int64], base: Slice[T]) -> Optional[&Slice[T]] { checkedSubslice(base, index.resolve(base.count)) }
    public static func lendMutatingChecked(consuming index: ClosedRange[Int64], mutating base: Slice[T]) -> Optional[&mutating Slice[T]] { checkedSubsliceMutating(base, index.resolve(base.count)) }
}

extend RangeFrom[Int64]: SliceIndex[T] {
    type SliceOutput = Slice[T]
    public static func lend(consuming index: RangeFrom[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).value }
    public static func lendMutating(consuming index: RangeFrom[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).mutatingValue }
    public static func lendUnchecked(consuming index: RangeFrom[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, index.resolve(base.count)).value }
    public static func lendMutatingUnchecked(consuming index: RangeFrom[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, index.resolve(base.count)).mutatingValue }
    public static func lendChecked(consuming index: RangeFrom[Int64], base: Slice[T]) -> Optional[&Slice[T]] { checkedSubslice(base, index.resolve(base.count)) }
    public static func lendMutatingChecked(consuming index: RangeFrom[Int64], mutating base: Slice[T]) -> Optional[&mutating Slice[T]] { checkedSubsliceMutating(base, index.resolve(base.count)) }
}

extend RangeUpTo[Int64]: SliceIndex[T] {
    type SliceOutput = Slice[T]
    public static func lend(consuming index: RangeUpTo[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).value }
    public static func lendMutating(consuming index: RangeUpTo[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).mutatingValue }
    public static func lendUnchecked(consuming index: RangeUpTo[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, index.resolve(base.count)).value }
    public static func lendMutatingUnchecked(consuming index: RangeUpTo[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, index.resolve(base.count)).mutatingValue }
    public static func lendChecked(consuming index: RangeUpTo[Int64], base: Slice[T]) -> Optional[&Slice[T]] { checkedSubslice(base, index.resolve(base.count)) }
    public static func lendMutatingChecked(consuming index: RangeUpTo[Int64], mutating base: Slice[T]) -> Optional[&mutating Slice[T]] { checkedSubsliceMutating(base, index.resolve(base.count)) }
}

extend RangeThrough[Int64]: SliceIndex[T] {
    type SliceOutput = Slice[T]
    public static func lend(consuming index: RangeThrough[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).value }
    public static func lendMutating(consuming index: RangeThrough[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, checkedSliceRange(index.resolve(base.count), base.count)).mutatingValue }
    public static func lendUnchecked(consuming index: RangeThrough[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, index.resolve(base.count)).value }
    public static func lendMutatingUnchecked(consuming index: RangeThrough[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, index.resolve(base.count)).mutatingValue }
    public static func lendChecked(consuming index: RangeThrough[Int64], base: Slice[T]) -> Optional[&Slice[T]] { checkedSubslice(base, index.resolve(base.count)) }
    public static func lendMutatingChecked(consuming index: RangeThrough[Int64], mutating base: Slice[T]) -> Optional[&mutating Slice[T]] { checkedSubsliceMutating(base, index.resolve(base.count)) }
}

extend Range[Int64]: SliceClampable[T] {
    public static func lendWithin(consuming range: Range[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, clampSliceRange(range, base.count)).value }
    public static func lendMutatingWithin(consuming range: Range[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, clampSliceRange(range, base.count)).mutatingValue }
}

extend ClosedRange[Int64]: SliceClampable[T] {
    public static func lendWithin(consuming range: ClosedRange[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).value }
    public static func lendMutatingWithin(consuming range: ClosedRange[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).mutatingValue }
}

extend RangeFrom[Int64]: SliceClampable[T] {
    public static func lendWithin(consuming range: RangeFrom[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).value }
    public static func lendMutatingWithin(consuming range: RangeFrom[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).mutatingValue }
}

extend RangeUpTo[Int64]: SliceClampable[T] {
    public static func lendWithin(consuming range: RangeUpTo[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).value }
    public static func lendMutatingWithin(consuming range: RangeUpTo[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).mutatingValue }
}

extend RangeThrough[Int64]: SliceClampable[T] {
    public static func lendWithin(consuming range: RangeThrough[Int64], base: Slice[T]) -> &Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).value }
    public static func lendMutatingWithin(consuming range: RangeThrough[Int64], mutating base: Slice[T]) -> &mutating Slice[T] { subslice(base, clampSliceRange(range.resolve(base.count), base.count)).mutatingValue }
}

// ============================================================================
// HELPERS
// ============================================================================

/// A pointer to the slice of `base`'s elements `range` (not checked).
fileprivate func subslice[T](base: Slice[T], range: Range[Int64]) -> Pointer[Slice[T]] {
    Slice.at(base.pointer.offset(by: range.start), range.end - range.start)
}

/// The slice of `base`'s elements `range`, or `.None` when it doesn't fit.
fileprivate func checkedSubslice[T](base: Slice[T], range: Range[Int64]) -> Optional[&Slice[T]] {
    if range.start < 0 or range.end > base.count or range.start > range.end {
        return .None;
    }
    .Some(subslice(base, range).value)
}

/// Like `checkedSubslice`, lent mutably.
fileprivate func checkedSubsliceMutating[T](mutating base: Slice[T], range: Range[Int64]) -> Optional[&mutating Slice[T]] {
    if range.start < 0 or range.end > base.count or range.start > range.end {
        return .None;
    }
    .Some(subslice(base, range).mutatingValue)
}

/// `range`, after checking it fits `0..<count`.
fileprivate func checkedSliceRange(range: Range[Int64], count: Int64) -> Range[Int64] {
    if range.start < 0 or range.end > count or range.start > range.end {
        fatalError("Slice range out of bounds");
    }
    range
}

/// `index` modulo `count` (negative counts from the end), or `-1` when
/// `count` is 0.
fileprivate func wrapSliceIndex(index: Int64, count: Int64) -> Int64 {
    if count == 0 { return -1; }
    let i = index % count;
    if i < 0 { return i + count; }
    i
}

/// `index` saturated into `0..<count`, or `-1` when `count` is 0.
fileprivate func clampSliceIndex(index: Int64, count: Int64) -> Int64 {
    if count == 0 { return -1; }
    if index < 0 { return 0; }
    if index >= count { return count - 1; }
    index
}

/// `range` clamped into `0..<count`.
fileprivate func clampSliceRange(range: Range[Int64], count: Int64) -> Range[Int64] {
    var lo = range.start;
    var hi = range.end;
    if lo < 0 { lo = 0 }
    if hi > count { hi = count }
    if lo > hi { lo = hi }
    Range[Int64](lo, hi)
}
