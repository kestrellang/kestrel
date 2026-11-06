// In-place algorithms on Slice[T], and Array's reference iterators

module std.collections

import std.core.(Bool, Comparable, fatalError)
import std.numeric.(Int64, UInt64, RandomNumberGenerator, SystemRandom)
import std.result.(Optional)
import std.memory.(UnsafeSlice, Pointer)
import std.iter.(Iterator)
import std.collections.(Array, Slice)

// ============================================================================
// IN-PLACE ALGORITHMS
// ============================================================================
// Writes through `&mutating Slice[T]`. `Array`'s in-place methods delegate
// through `asMutableSlice()`, which runs the copy-on-write check first, so
// `xs(1..<4).sort()` and `xs.sort()` share one implementation.

extend Slice[T] where T: Copyable {
    /// Swaps elements `i` and `j` of the slice. Panics if either is out of
    /// range.
    public mutating func swap(at i: Int64, with j: Int64) {
        let n = self.count;
        if i < 0 or i >= n or j < 0 or j >= n {
            fatalError("Slice.swap: index out of bounds");
        }
        if i == j {
            return
        }
        swapRaw(self.asUnsafeSlice(), i, j)
    }

    /// Reverses the slice in place. O(n).
    public mutating func reverse() {
        let raw = self.asUnsafeSlice();
        reverseRaw(raw, 0, raw.count)
    }

    /// Rotates the slice left by `amount` (negative rotates right; the
    /// amount wraps modulo `count`). Three-reversal algorithm: O(n) time,
    /// O(1) space.
    public mutating func rotate(by amount: Int64) {
        let raw = self.asUnsafeSlice();
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
        let raw = self.asUnsafeSlice();
        var i = raw.count - 1;
        while i > 0 {
            let j = Int64(truncating: rng.nextUInt64(below: UInt64(truncating: i) + 1));
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
        let raw = self.asUnsafeSlice();
        let p = raw.pointer;
        var lo: Int64 = 0;
        var hi: Int64 = raw.count - 1;
        while true {
            while lo < raw.count {
                let matches = predicate(unsafe p.offset(by: lo).value);
                if matches == false { break };
                lo = lo + 1
            }
            while hi >= 0 {
                let matches = predicate(unsafe p.offset(by: hi).value);
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
        let raw = self.asUnsafeSlice();
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
}

extend Slice[T] where T: Copyable {
    /// Sets every element of the slice to `value`.
    public mutating func fill(with value: T) {
        var raw = self.asUnsafeSlice();
        for i in 0..<raw.count {
            raw(unchecked: i) = value;
        }
    }
}

extend Slice[T] where T: Comparable {
    /// Sorts the slice in place, ascending (introsort; see `sort(by:)`).
    public mutating func sort() {
        self.sort(by: { (a, b) in a < b })
    }
}

// ============================================================================
// IN-PLACE KERNELS
// ============================================================================
// One implementation of each in-place algorithm, over the raw view of a
// slice reached through `&mutating` (so already copy-on-write checked).

/// Swaps elements `i` and `j` (unchecked).
fileprivate func swapRaw[T](s: UnsafeSlice[T], i: Int64, j: Int64) {
    let a = s.pointer.offset(by: i);
    let b = s.pointer.offset(by: j);
    // Move, not `read()`: `write` drops nothing, so a copy leaked.
    let t = unsafe a.take();
    unsafe a.write(b.take());
    unsafe b.write(t)
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

// ============================================================================
// ARRAY REFERENCE ITERATORS
// ============================================================================

extend Array[T] {
    /// Iterator over SHARED REFERENCES (`&T`) to the elements, in place —
    /// no copies, no clones. It borrows the array, so it can't outlive it.
    ///
    /// # Examples
    ///
    /// ```
    /// let words = ["alpha", "beta"];
    /// for w in words.refs() {
    ///     print(w.len());   // reads in place — no element copy
    /// }
    /// ```
    public func refs() -> ArrayRefsIterator[T] {
        let b = &self;
        ArrayRefsIterator(base: b, index: 0, end: self.count)
    }

    /// Iterator over MUTABLE REFERENCES (`&mutating T`) to the elements,
    /// for in-place updates without writeback. Each write runs the
    /// copy-on-write check, so writes never leak into shared storage.
    ///
    /// # Examples
    ///
    /// ```
    /// var xs = [1, 2, 3];
    /// for x in xs.mutatingRefs() {
    ///     x += 1;
    /// }
    /// // xs == [2, 3, 4]
    /// ```
    public mutating func mutatingRefs() -> ArrayMutatingRefsIterator[T] {
        let b = &mutating self;
        ArrayMutatingRefsIterator(base: b, index: 0, end: self.count)
    }
}

// ============================================================================
// REFERENCE ITERATORS
// ============================================================================

/// Iterator over `&T` references to a range of an array's elements.
/// Returned by `Array.refs()`. It borrows the
/// array and reads each element through it, so a reference can't
/// outlive the array. Stops early if the array gets shorter.
public struct ArrayRefsIterator[T]: Iterator {
    type Item = &T

    fileprivate let base: &Array[T]
    fileprivate var index: Int64
    fileprivate let end: Int64

    public mutating func next() -> Optional[&T] {
        if self.index >= self.end or self.index >= self.base.count {
            return .None
        }
        let i = self.index;
        self.index = i + 1;
        let r = &self.base(i);
        let o: Optional[&T] = .Some(r);
        o
    }
}

/// Iterator over `&mutating T` references to an array's elements, for
/// in-place updates (`for x in xs.mutatingRefs() { x += 1 }`). Returned by
/// `Array.mutatingRefs()`. Each write goes through the array's own
/// element accessor, so it runs the copy-on-write check: a copy of the
/// array taken during the loop keeps its old contents.
public struct ArrayMutatingRefsIterator[T]: Iterator {
    type Item = &mutating T

    fileprivate let base: &mutating Array[T]
    fileprivate var index: Int64
    fileprivate let end: Int64

    public mutating func next() -> Optional[&mutating T] {
        if self.index >= self.end or self.index >= self.base.count {
            return .None
        }
        let i = self.index;
        self.index = i + 1;
        let r = &mutating self.base(i);
        let o: Optional[&mutating T] = .Some(r);
        o
    }
}
