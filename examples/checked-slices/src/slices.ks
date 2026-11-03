module CheckedSlices

// Checked slices: a slice is a tracked reference to its array plus a range.
//
// Because `base` is a reference, the compiler treats every slice exactly
// like a `&T`: it can't be returned past the array it borrows (E494), stored
// somewhere that outlives the array (E509), or kept in a global (E505).
//
// Every access goes through the array itself, never through a cached element
// pointer. Growing or replacing the array while a slice is live is therefore
// safe: the slice sees the new buffer, and a range that no longer fits
// panics instead of reading freed memory.

/// A read-only view of `count` consecutive elements of an array.
public struct ArraySlice[T] {
    let base: &Array[T]
    let start: Int64
    /// Number of elements in the slice.
    public let count: Int64

    /// `true` when the slice has no elements.
    public var isEmpty: Bool { self.count == 0 }

    /// Element `index` of the slice, in place (no copy). Panics when
    /// `index` is outside the slice or the array shrank under it.
    public subscript(index: Int64) -> T {
        ref {
            checkIndex(index, self.count);
            self.base(at: self.start + index)
        }
    }

    /// A narrower slice of the same array: elements `[from, from + count)`
    /// of this slice. It borrows the same array, not this slice.
    public func slice(from from: Int64, count count: Int64) -> ArraySlice[T] {
        checkRange(from, count, self.count);
        ArraySlice(base: self.base, start: self.start + from, count: count)
    }
}

/// A writable view of `count` consecutive elements of an array.
///
/// Writes go through the array's own element accessor, so they run its
/// copy-on-write check: a copy of the array taken before the write keeps
/// its old contents. A mutable slice can change elements but never the
/// array's length.
public struct MutableArraySlice[T] {
    let base: &mutating Array[T]
    let start: Int64
    /// Number of elements in the slice.
    public let count: Int64

    /// Element `index` of the slice, in place.
    public subscript(index: Int64) -> T {
        ref {
            checkIndex(index, self.count);
            self.base(at: self.start + index)
        }
    }

    /// Overwrites element `index` with `value`.
    public func update(at index: Int64, to value: T) where T: Copyable {
        checkIndex(index, self.count);
        self.base(at: self.start + index) = value;
    }

    /// Swaps elements `i` and `j` of the slice.
    public func swapAt(i: Int64, j: Int64) {
        checkIndex(i, self.count);
        checkIndex(j, self.count);
        self.base.swap(at: self.start + i, with: self.start + j);
    }

    /// Sets every element of the slice to `value`.
    public func fill(with value: T) where T: Copyable {
        for i in 0..<self.count {
            self.base(at: self.start + i) = value;
        }
    }

    /// Reverses the slice in place.
    public func reverse() {
        var lo: Int64 = 0;
        var hi = self.count - 1;
        while lo < hi {
            self.swapAt(lo, hi);
            lo += 1;
            hi -= 1;
        }
    }

    /// Sorts the slice in place, ascending (insertion sort).
    public func sort() where T: Comparable {
        for i in 1..<self.count {
            var j = i;
            while j > 0 and self(j) < self(j - 1) {
                self.swapAt(j, j - 1);
                j -= 1;
            }
        }
    }

    /// A read-only view of the same elements.
    public func readOnly() -> ArraySlice[T] {
        let b = &self.base;
        ArraySlice(base: b, start: self.start, count: self.count)
    }
}

extend Array[T] {
    /// A read-only slice of elements `[from, from + count)`.
    public func slice(from from: Int64, count count: Int64) -> ArraySlice[T] {
        checkRange(from, count, self.count);
        let b = &self;
        ArraySlice(base: b, start: from, count: count)
    }

    /// A writable slice of elements `[from, from + count)`.
    public mutating func mutatingSlice(from from: Int64, count count: Int64) -> MutableArraySlice[T] {
        checkRange(from, count, self.count);
        let b = &mutating self;
        MutableArraySlice(base: b, start: from, count: count)
    }
}

func checkIndex(index: Int64, count: Int64) {
    if index < 0 or index >= count {
        fatalError("slice index out of range");
    }
}

func checkRange(from: Int64, count: Int64, limit: Int64) {
    if from < 0 or count < 0 or from + count > limit {
        fatalError("slice range out of bounds");
    }
}
