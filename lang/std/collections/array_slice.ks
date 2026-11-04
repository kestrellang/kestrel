// ArraySlice[T] / MutableArraySlice[T] - checked views into an Array

module std.collections

import std.core.(Bool, Equatable, Range, ClosedRange, RangeFrom, RangeUpTo, RangeThrough, fatalError)
import std.numeric.(Int64, UInt64, RandomNumberGenerator, SystemRandom)
import std.result.(Optional)
import std.memory.(UnsafeSlice, Pointer)
import std.iter.(Iterable)
import std.collections.(Array, ArrayIterator, Slice, SeqIndex, SeqClampable, SeqWrappable, SeqRange)

// ============================================================================
// ARRAY SLICE
// ============================================================================

/// A read-only view of a run of elements of an `Array`.
///
/// The slice holds a reference to the array plus a range, and reads
/// through the array on every access. Because it holds a reference, the
/// compiler keeps it from outliving the array: it can't be returned past
/// the array's scope (E494), stored somewhere that outlives the array
/// (E509), or kept in a global (E505).
///
/// Changing the array while a slice is live is allowed and safe. The slice
/// sees the array's current buffer, and an access whose range no longer
/// fits the array panics instead of reading freed memory.
///
/// # Examples
///
/// ```
/// let xs = [1, 2, 3, 4, 5];
/// let middle = xs(1..<4);     // ArraySlice: [2, 3, 4]
/// middle(0);                  // 2
/// middle(1..<3);              // [3, 4], a slice of the same array
/// ```
@builtin(.SliceStruct)
public struct ArraySlice[T] {
    fileprivate let base: &Array[T]
    fileprivate let start: Int64
    fileprivate let len: Int64

    /// Element count.
    public var count: Int64 { self.len }

    /// `true` when `count == 0`.
    public var isEmpty: Bool { self.len == 0 }

    /// Element `index` of the slice, in place (no copy). Panics when the
    /// index is outside the slice.
    public subscript(at index: Int64) -> T {
        ref {
            if index < 0 or index >= self.len {
                fatalError("ArraySlice(at:): index out of bounds");
            }
            self.base(at: self.start + index)
        }
    }

    /// Element access and sub-slicing, like the array's own subscripts.
    /// A range gives a narrower slice of the same array.
    public subscript[I](index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArray(base: self.base, start: self.start, count: self.len) }
    }

    /// Like `self(index)`, but `.None` instead of a panic when out of range.
    public subscript[I](checked index: I) -> I.ArrayOutput? where I: ArrayIndex[T] {
        get { index.readArrayChecked(base: self.base, start: self.start, count: self.len) }
    }

    /// Like `self(index)`, without the bounds check against the slice.
    public subscript[I](unchecked index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArrayUnchecked(base: self.base, start: self.start, count: self.len) }
    }

    /// Like `self(index)`, with the index or range clamped into the slice.
    public subscript[I](clamped index: I) -> I.ArrayClampedOutput where I: ArrayClampable[T] {
        get { index.readArrayClamped(base: self.base, start: self.start, count: self.len) }
    }

    /// The first `count` elements. Panics if `count > self.count`.
    public func prefix(count: Int64) -> ArraySlice[T] { self(..<count) }

    /// The last `count` elements. Panics if `count > self.count`.
    public func suffix(count: Int64) -> ArraySlice[T] { self((self.len - count)..) }

    /// Everything after the first `count` elements.
    public func drop(first count: Int64) -> ArraySlice[T] { self(count..) }

    /// Everything before the last `count` elements.
    public func drop(last count: Int64) -> ArraySlice[T] { self(..<(self.len - count)) }

    /// Forward iterator over the elements. It shares the array's storage,
    /// so it sees the elements as they are now, like `Array.iter()`.
    public func iter() -> ArrayIterator[T] {
        self.base.iterRange(start: self.start, count: liveCount(self.base, self.start, self.len))
    }

    /// An owned copy of the elements.
    public func toArray() -> Array[T] {
        var out: Array[T] = [];
        for x in self.iter() {
            out.append(x);
        }
        out
    }
}

/// `Slice[T]` conformance: every read-only algorithm on `Slice` works on an
/// `ArraySlice` too.
extend ArraySlice[T]: Slice[T], Iterable {
    type Item = T
    type TargetIterator = ArrayIterator[T]

    /// The elements as a raw `(pointer, count)` view, valid until the array
    /// next changes. Panics if the slice no longer fits the array.
    public func asUnsafeSlice() -> UnsafeSlice[T] {
        let n = liveCount(self.base, self.start, self.len);
        UnsafeSlice(pointer: self.base.asUnsafeSlice().pointer.offset(by: self.start), count: n)
    }
}

/// `ArrayMatchable` conformance so a slice can appear in array patterns.
/// `..rest` binds a narrower slice of the same array.
extend ArraySlice[T]: ArrayMatchable {
    type Element = T
    type Rest = ArraySlice[T]

    public func matchLength() -> Int64 { self.len }

    public func matchGet(index: Int64) -> T {
        self.asUnsafeSlice().pointer.offset(by: index).read()
    }

    public func matchSlice(from: Int64, to: Int64) -> ArraySlice[T] {
        ArraySlice(base: self.base, start: self.start + from, len: to - from)
    }
}

/// Element-wise equality (`isEqual` comes from `extend Slice[T] where T: Equatable`).
extend ArraySlice[T]: Equatable where T: Equatable { }

// ============================================================================
// MUTABLE ARRAY SLICE
// ============================================================================

/// A writable view of a run of elements of an `Array`, from
/// `Array.mutatingSlice(_:)`.
///
/// Writes go through the array, so they run its copy-on-write check: a copy
/// of the array taken before the write keeps its old contents. A mutable
/// slice can change elements but never the array's length.
///
/// # Examples
///
/// ```
/// var xs = [5, 3, 9, 1, 7];
/// xs.mutatingSlice(1..<4).sort();   // xs == [5, 1, 3, 9, 7]
/// ```
public struct MutableArraySlice[T] {
    fileprivate let base: &mutating Array[T]
    fileprivate let start: Int64
    fileprivate let len: Int64

    /// Element count.
    public var count: Int64 { self.len }

    /// `true` when `count == 0`.
    public var isEmpty: Bool { self.len == 0 }

    /// Element `index` of the slice, in place (read-only; write with
    /// `slice(index) = value`).
    public subscript(at index: Int64) -> T {
        ref {
            if index < 0 or index >= self.len {
                fatalError("MutableArraySlice(at:): index out of bounds");
            }
            self.base(at: self.start + index)
        }
    }

    /// Element access and writes. A range reads as a read-only slice.
    public subscript[I](index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArray(base: self.base, start: self.start, count: self.len) }
        set { index.writeArray(base: self.base, start: self.start, count: self.len, value: newValue) }
    }

    /// Like `self(index)`, but `.None` instead of a panic when out of range.
    public subscript[I](checked index: I) -> I.ArrayOutput? where I: ArrayIndex[T] {
        get { index.readArrayChecked(base: self.base, start: self.start, count: self.len) }
    }

    /// Like `self(index)`, without the bounds check against the slice.
    public subscript[I](unchecked index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArrayUnchecked(base: self.base, start: self.start, count: self.len) }
        set { index.writeArrayUnchecked(base: self.base, start: self.start, count: self.len, value: newValue) }
    }

    /// Like `self(index)`, with the index or range clamped into the slice.
    public subscript[I](clamped index: I) -> I.ArrayClampedOutput where I: ArrayClampable[T] {
        get { index.readArrayClamped(base: self.base, start: self.start, count: self.len) }
        set { index.writeArrayClamped(base: self.base, start: self.start, count: self.len, value: newValue) }
    }

    /// A narrower writable slice of the same array: elements `range` of
    /// this slice.
    public mutating func mutatingSlice[R](range: R) -> MutableArraySlice[T] where R: SeqRange {
        let r = checkedRange(range.resolve(self.len), self.len);
        MutableArraySlice(base: self.base, start: self.start + r.start, len: r.end - r.start)
    }

    /// A read-only view of the same elements.
    public func readOnly() -> ArraySlice[T] {
        let b = &self.base;
        ArraySlice(base: b, start: self.start, len: self.len)
    }

    /// Swaps elements `i` and `j` of the slice. Panics if either is out of
    /// range.
    public mutating func swap(at i: Int64, with j: Int64) {
        if i < 0 or i >= self.len or j < 0 or j >= self.len {
            fatalError("MutableArraySlice.swap: index out of bounds");
        }
        if i == j {
            return
        }
        swapRaw(self.asUnsafeMutableSlice(), i, j)
    }

    /// Reverses the slice in place. O(n).
    public mutating func reverse() {
        let raw = self.asUnsafeMutableSlice();
        reverseRaw(raw, 0, raw.count)
    }

    /// Rotates the slice left by `amount` (negative rotates right; the
    /// amount wraps modulo `count`). Three-reversal algorithm: O(n) time,
    /// O(1) space.
    public mutating func rotate(by amount: Int64) {
        let raw = self.asUnsafeMutableSlice();
        let n = raw.count;
        if n <= 1 {
            return
        }
        var k = amount % n;
        if k < 0 {
            k = k + n
        }
        if k == 0 {
            return
        }
        reverseRaw(raw, 0, k);
        reverseRaw(raw, k, n);
        reverseRaw(raw, 0, n)
    }

    /// Shuffles the slice in place with `rng` (Fisher-Yates: every
    /// permutation equally likely for a uniform generator; a seeded `rng`
    /// gives a deterministic shuffle).
    public mutating func shuffle(mutating using rng: some RandomNumberGenerator) {
        let raw = self.asUnsafeMutableSlice();
        var i = raw.count - 1;
        while i > 0 {
            let j = Int64(from: rng.nextUInt64(below: UInt64(from: i) + 1));
            swapRaw(raw, i, j);
            i = i - 1
        }
    }

    /// Shuffles the slice in place with a fresh system RNG.
    public mutating func shuffle() {
        var rng = SystemRandom();
        self.shuffle(using: rng)
    }

    /// Reorders the slice so every element matching `predicate` comes
    /// first; returns how many match (the index of the first non-match).
    /// Unstable.
    public mutating func partition(by predicate: (T) -> Bool) -> Int64 {
        let raw = self.asUnsafeMutableSlice();
        let p = raw.pointer;
        var lo: Int64 = 0;
        var hi: Int64 = raw.count - 1;
        while true {
            while lo < raw.count {
                let matches = p.offset(by: lo).with { (element) in predicate(element) };
                if matches == false { break };
                lo = lo + 1
            }
            while hi >= 0 {
                let matches = p.offset(by: hi).with { (element) in predicate(element) };
                if matches { break };
                hi = hi - 1
            }
            if lo >= hi {
                break
            }
            swapRaw(raw, lo, hi);
            lo = lo + 1;
            hi = hi - 1
        }
        lo
    }

    /// Sorts the slice in place with a `<`-style comparator. Introsort
    /// (quicksort, heapsort past 2·log₂(n) depth, insertion sort for runs
    /// of ≤ 16): O(n log n) worst case, unstable.
    public mutating func sort(by comparator: (T, T) -> Bool) {
        let raw = self.asUnsafeMutableSlice();
        let n = raw.count;
        if n <= 1 {
            return
        }
        var depthLimit: Int64 = 0;
        var v = n;
        while v > 1 { v = v / 2; depthLimit = depthLimit + 1 }
        introsortLoop(raw, 0, n, depthLimit * 2, comparator)
    }

    /// Sorts the slice in place by an extracted `Comparable` key.
    public mutating func sort[K](consuming byKey key: (T) -> K) where K: Comparable {
        self.sort(by: { (a, b) in key(a) < key(b) })
    }

    /// Forward iterator over the elements, as they are now.
    public func iter() -> ArrayIterator[T] {
        self.base.iterRange(start: self.start, count: liveCount(self.base, self.start, self.len))
    }
}

extend MutableArraySlice[T] where T: Copyable {
    /// Sets every element of the slice to `value`.
    public mutating func fill(with value: T) {
        var raw = self.asUnsafeMutableSlice();
        for i in 0..<raw.count {
            raw(unchecked: i) = value;
        }
    }
}

extend MutableArraySlice[T] where T: Comparable {
    /// Sorts the slice in place, ascending (introsort; see `sort(by:)`).
    public mutating func sort() {
        self.sort(by: { (a, b) in a < b })
    }
}

// ============================================================================
// IN-PLACE KERNELS
// ============================================================================
// One implementation of each in-place algorithm, over a raw view the caller
// has already copy-on-write checked (`asUnsafeMutableSlice()`).
// `MutableArraySlice` owns the public methods; `Array`'s in-place methods
// delegate through `asMutableSlice()`.

/// Swaps elements `i` and `j` (unchecked).
fileprivate func swapRaw[T](s: UnsafeSlice[T], i: Int64, j: Int64) {
    let a = s.pointer.offset(by: i);
    let b = s.pointer.offset(by: j);
    let t = a.read();
    a.write(b.read());
    b.write(t)
}

/// Reverses `[lo, hi)` (unchecked).
fileprivate func reverseRaw[T](s: UnsafeSlice[T], lo: Int64, hi: Int64) {
    var left = lo;
    var right = hi - 1;
    while left < right {
        swapRaw(s, left, right);
        left = left + 1;
        right = right - 1
    }
}

fileprivate func introsortLoop[T](s: UnsafeSlice[T], lo: Int64, hi: Int64, depth: Int64, cmp: (T, T) -> Bool) {
    if hi - lo <= 16 {
        insertionSortRange(s, lo, hi, cmp);
        return
    }
    if depth == 0 {
        heapsortRange(s, lo, hi, cmp);
        return
    }
    let p = partitionRange(s, lo, hi, cmp);
    introsortLoop(s, lo, p, depth - 1, cmp);
    introsortLoop(s, p + 1, hi, depth - 1, cmp)
}

/// Lomuto partition with a median-of-three pivot.
fileprivate func partitionRange[T](s: UnsafeSlice[T], lo: Int64, hi: Int64, cmp: (T, T) -> Bool) -> Int64 {
    var v = s;
    let mid = lo + (hi - lo) / 2;
    let last = hi - 1;
    // Order lo, mid, last so the median lands at mid.
    if cmp(v(unchecked: mid), v(unchecked: lo)) { swapRaw(v, lo, mid) }
    if cmp(v(unchecked: last), v(unchecked: lo)) { swapRaw(v, lo, last) }
    if cmp(v(unchecked: last), v(unchecked: mid)) { swapRaw(v, mid, last) }
    // Move the median to the pivot slot (last).
    swapRaw(v, mid, last);
    let pivot = v(unchecked: last);
    var i = lo;
    for j in lo..<last {
        if cmp(v(unchecked: j), pivot) {
            swapRaw(v, i, j);
            i = i + 1
        }
    }
    swapRaw(v, i, last);
    i
}

fileprivate func insertionSortRange[T](s: UnsafeSlice[T], lo: Int64, hi: Int64, cmp: (T, T) -> Bool) {
    var v = s;
    for i in (lo + 1)..<hi {
        let key = v(unchecked: i);
        var j = i - 1;
        while j >= lo and cmp(key, v(unchecked: j)) {
            v(unchecked: j + 1) = v(unchecked: j);
            j = j - 1
        }
        v(unchecked: j + 1) = key
    }
}

/// Max-heap sort of `[lo, hi)`.
fileprivate func heapsortRange[T](s: UnsafeSlice[T], lo: Int64, hi: Int64, cmp: (T, T) -> Bool) {
    let n = hi - lo;
    var i = n / 2 - 1;
    while i >= 0 {
        siftDown(s, lo, i, n, cmp);
        i = i - 1
    }
    var end = n - 1;
    while end > 0 {
        swapRaw(s, lo, lo + end);
        siftDown(s, lo, 0, end, cmp);
        end = end - 1
    }
}

fileprivate func siftDown[T](s: UnsafeSlice[T], lo: Int64, start: Int64, n: Int64, cmp: (T, T) -> Bool) {
    var idx = start;
    while true {
        var largest = idx;
        let left = 2 * idx + 1;
        let right = 2 * idx + 2;
        if left < n and cmp(s(unchecked: lo + largest), s(unchecked: lo + left)) {
            largest = left
        }
        if right < n and cmp(s(unchecked: lo + largest), s(unchecked: lo + right)) {
            largest = right
        }
        if largest == idx { return }
        swapRaw(s, lo + idx, lo + largest);
        idx = largest
    }
}

/// `Slice[T]` conformance: the read-only algorithms work on a mutable
/// slice too.
extend MutableArraySlice[T]: Slice[T], Iterable {
    type Item = T
    type TargetIterator = ArrayIterator[T]

    public func asUnsafeSlice() -> UnsafeSlice[T] {
        let n = liveCount(self.base, self.start, self.len);
        UnsafeSlice(pointer: self.base.asUnsafeSlice().pointer.offset(by: self.start), count: n)
    }
}

extend MutableArraySlice[T] {
    /// Raw view for writing the elements through a pointer (a syscall
    /// buffer, an FFI out-param). Runs the copy-on-write check first, so
    /// writes never reach storage another array shares.
    public mutating func asUnsafeMutableSlice() -> UnsafeSlice[T] {
        self.base.ensureUnique();
        self.asUnsafeSlice()
    }
}

// ============================================================================
// ARRAY: SLICING
// ============================================================================

extend Array[T] {
    /// Element access and sub-slicing: `xs(2)` is an element, `xs(1..<3)`
    /// is an `ArraySlice` of the array. Writes run the copy-on-write check.
    public subscript[I](index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArray(base: self, start: 0, count: self.count) }
        set { index.writeArray(base: self, start: 0, count: self.count, value: newValue) }
    }

    /// Like `xs(index)`, but `.None` instead of a panic when out of range.
    public subscript[I](checked index: I) -> I.ArrayOutput? where I: ArrayIndex[T] {
        get { index.readArrayChecked(base: self, start: 0, count: self.count) }
    }

    /// Like `xs(index)`, without the bounds check.
    public subscript[I](unchecked index: I) -> I.ArrayOutput where I: ArrayIndex[T] {
        get { index.readArrayUnchecked(base: self, start: 0, count: self.count) }
        set { index.writeArrayUnchecked(base: self, start: 0, count: self.count, value: newValue) }
    }

    /// Like `xs(index)`, with the index or range clamped into the array.
    public subscript[I](clamped index: I) -> I.ArrayClampedOutput where I: ArrayClampable[T] {
        get { index.readArrayClamped(base: self, start: 0, count: self.count) }
        set { index.writeArrayClamped(base: self, start: 0, count: self.count, value: newValue) }
    }

    /// Element at `index` modulo `count` (negative indices count from the
    /// end), or `.None` for an empty array. A write to an empty array does
    /// nothing.
    public subscript(wrapped index: Int64) -> T? {
        get { index.readSeqWrapped(from: self.asUnsafeSlice()) }
        set {
            self.ensureUnique();
            index.writeSeqWrapped(to: self.asUnsafeSlice(), with: newValue)
        }
    }

    /// A read-only slice of the whole array.
    public func asSlice() -> ArraySlice[T] {
        let b = &self;
        ArraySlice(base: b, start: 0, len: self.count)
    }

    /// A writable slice of the whole array.
    public mutating func asMutableSlice() -> MutableArraySlice[T] {
        let n = self.count;
        let b = &mutating self;
        MutableArraySlice(base: b, start: 0, len: n)
    }

    /// A writable slice of elements `range`.
    public mutating func mutatingSlice[R](range: R) -> MutableArraySlice[T] where R: SeqRange {
        let r = checkedRange(range.resolve(self.count), self.count);
        let b = &mutating self;
        MutableArraySlice(base: b, start: r.start, len: r.end - r.start)
    }

    /// The first `count` elements. Panics if `count > self.count`.
    public func prefix(count: Int64) -> ArraySlice[T] { self(..<count) }

    /// The last `count` elements. Panics if `count > self.count`.
    public func suffix(count: Int64) -> ArraySlice[T] { self((self.count - count)..) }

    /// Everything after the first `count` elements.
    public func drop(first count: Int64) -> ArraySlice[T] { self(count..) }

    /// Everything before the last `count` elements.
    public func drop(last count: Int64) -> ArraySlice[T] { self(..<(self.count - count)) }
}

// ============================================================================
// INDEX PROTOCOLS
// ============================================================================

/// An index into an array window `[start, start + count)` of `base`: an
/// `Int64` reads an element, a range reads an `ArraySlice` of `base`.
public protocol ArrayIndex[T] {
    type ArrayOutput
    func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArrayOutput
    func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArrayOutput?
    func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArrayOutput
    func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArrayOutput)
    func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArrayOutput)
}

/// The clamped form of `ArrayIndex`.
public protocol ArrayClampable[T] {
    type ArrayClampedOutput
    func readArrayClamped(base base: Array[T], start start: Int64, count count: Int64) -> ArrayClampedOutput
    func writeArrayClamped(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArrayClampedOutput)
}

extend Int64: ArrayIndex[T] {
    type ArrayOutput = T

    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> T {
        self.readSeq(from: window(base, start, count))
    }

    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> T? {
        self.readSeqChecked(from: window(base, start, count))
    }

    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> T {
        self.readSeqUnchecked(from: window(base, start, count))
    }

    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: T) {
        base.ensureUnique();
        self.writeSeq(to: window(base, start, count), with: value)
    }

    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: T) {
        base.ensureUnique();
        self.writeSeqUnchecked(to: window(base, start, count), with: value)
    }
}

extend Int64: ArrayClampable[T] {
    type ArrayClampedOutput = T?

    public func readArrayClamped(base base: Array[T], start start: Int64, count count: Int64) -> T? {
        self.readSeqClamped(from: window(base, start, count))
    }

    public func writeArrayClamped(mutating base base: Array[T], start start: Int64, count count: Int64, value value: T?) {
        base.ensureUnique();
        self.writeSeqClamped(to: window(base, start, count), with: value)
    }
}

extend Range[Int64]: ArrayIndex[T] {
    type ArrayOutput = ArraySlice[T]
    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { sliceOf(base, start, count, self) }
    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T]? { checkedSliceOf(base, start, count, self) }
    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { uncheckedSliceOf(base, start, count, self) }
    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
}

extend ClosedRange[Int64]: ArrayIndex[T] {
    type ArrayOutput = ArraySlice[T]
    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { sliceOf(base, start, count, self) }
    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T]? { checkedSliceOf(base, start, count, self) }
    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { uncheckedSliceOf(base, start, count, self) }
    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
}

extend RangeFrom[Int64]: ArrayIndex[T] {
    type ArrayOutput = ArraySlice[T]
    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { sliceOf(base, start, count, self) }
    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T]? { checkedSliceOf(base, start, count, self) }
    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { uncheckedSliceOf(base, start, count, self) }
    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
}

extend RangeUpTo[Int64]: ArrayIndex[T] {
    type ArrayOutput = ArraySlice[T]
    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { sliceOf(base, start, count, self) }
    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T]? { checkedSliceOf(base, start, count, self) }
    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { uncheckedSliceOf(base, start, count, self) }
    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
}

extend RangeThrough[Int64]: ArrayIndex[T] {
    type ArrayOutput = ArraySlice[T]
    public func readArray(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { sliceOf(base, start, count, self) }
    public func readArrayChecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T]? { checkedSliceOf(base, start, count, self) }
    public func readArrayUnchecked(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] { uncheckedSliceOf(base, start, count, self) }
    public func writeArray(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
    public func writeArrayUnchecked(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) { writeRange(base, start, count, self, value) }
}

extend Range[Int64]: ArrayClampable[T] {
    type ArrayClampedOutput = ArraySlice[T]

    public func readArrayClamped(base base: Array[T], start start: Int64, count count: Int64) -> ArraySlice[T] {
        let r = clampRange(self, count);
        let b = &base;
        ArraySlice(base: b, start: start + r.start, len: r.end - r.start)
    }

    public func writeArrayClamped(mutating base base: Array[T], start start: Int64, count count: Int64, value value: ArraySlice[T]) {
        writeRange(base, start, count, clampRange(self, count), value)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// The window's elements as a raw view, after checking the window still
/// fits the array (it may have shrunk since the slice was made).
fileprivate func window[T](base: Array[T], start: Int64, count: Int64) -> UnsafeSlice[T] {
    let n = liveCount(base, start, count);
    UnsafeSlice(pointer: base.asUnsafeSlice().pointer.offset(by: start), count: n)
}

/// `count`, after checking `[start, start + count)` still fits `base`.
fileprivate func liveCount[T](base: Array[T], start: Int64, count: Int64) -> Int64 {
    if start + count > base.count {
        fatalError("slice range no longer fits its array");
    }
    count
}

/// `range` resolved against `count`, panicking when it falls outside.
fileprivate func checkedRange(range: Range[Int64], count: Int64) -> Range[Int64] {
    if range.start < 0 or range.end > count or range.start > range.end {
        fatalError("Range out of bounds");
    }
    range
}

/// `range` clamped into `0..<count`.
fileprivate func clampRange[R](range: R, count: Int64) -> Range[Int64] where R: SeqRange {
    let r = range.resolve(count);
    var lo = r.start;
    var hi = r.end;
    if lo < 0 { lo = 0 }
    if hi > count { hi = count }
    if lo > hi { lo = hi }
    Range[Int64](lo, hi)
}

fileprivate func sliceOf[T, R](base: Array[T], start: Int64, count: Int64, range: R) -> ArraySlice[T] where R: SeqRange {
    let r = checkedRange(range.resolve(count), count);
    let b = &base;
    ArraySlice(base: b, start: start + r.start, len: r.end - r.start)
}

fileprivate func checkedSliceOf[T, R](base: Array[T], start: Int64, count: Int64, range: R) -> ArraySlice[T]? where R: SeqRange {
    let r = range.resolve(count);
    if r.start < 0 or r.end > count or r.start > r.end {
        return .None
    }
    let b = &base;
    let s = ArraySlice(base: b, start: start + r.start, len: r.end - r.start);
    .Some(s)
}

fileprivate func uncheckedSliceOf[T, R](base: Array[T], start: Int64, count: Int64, range: R) -> ArraySlice[T] where R: SeqRange {
    let r = range.resolve(count);
    let b = &base;
    ArraySlice(base: b, start: start + r.start, len: r.end - r.start)
}

/// Copies `value` over elements `range` of the window. Panics when the
/// lengths differ.
fileprivate func writeRange[T, R](mutating base: Array[T], start: Int64, count: Int64, range: R, value: ArraySlice[T]) where R: SeqRange {
    let r = checkedRange(range.resolve(count), count);
    let n = r.end - r.start;
    if value.count != n {
        fatalError("Slice length doesn't match range length");
    }
    // Read the source before the write barrier: it may view this array.
    let src = value.asUnsafeSlice();
    base.ensureUnique();
    let dst = window(base, start, count).pointer.offset(by: r.start);
    var i = 0;
    while i < n {
        dst.offset(by: i).assign(src.pointer.offset(by: i).read());
        i += 1;
    }
}
