// Multi-pass lazy views over an array or array slice.
//
// Every view holds an `ArraySlice[T]`, so it borrows the array: it can't
// outlive it, and each access reads the array as it is now (bounds are
// re-checked, so a stale view panics rather than reading freed memory).
// The segments a view hands out are `ArraySlice[T]`s of the same array.
// Their `toArray()` copies segments into owned arrays, since an array
// can't hold slices.

module std.collections

import std.core.(Bool, Equatable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(ArraySlice)

// ============================================================================
// CHUNKS VIEW
// ============================================================================

/// Multi-pass lazy view over non-overlapping `chunkSize`-sized segments.
/// The last chunk is shorter when the count isn't a multiple of the size.
public struct ChunksView[T]: Iterable {
    type Item = ArraySlice[T]
    type TargetIterator = ChunksIterator[T]

    private var slice: ArraySlice[T]
    private var chunkSize: Int64

    public init(slice slice: ArraySlice[T], chunkSize chunkSize: Int64) {
        self.slice = slice;
        self.chunkSize = chunkSize;
    }

    public var count: Int64 {
        (self.slice.count + self.chunkSize - 1) / self.chunkSize
    }

    public var isEmpty: Bool { self.slice.count == 0 }

    /// Chunk `index`. Panics when out of range.
    public subscript(index: Int64) -> ArraySlice[T] {
        if index < 0 or index >= self.count {
            fatalError("ChunksView index out of bounds")
        }
        let start = index * self.chunkSize;
        var end = start + self.chunkSize;
        if end > self.slice.count {
            end = self.slice.count
        }
        self.slice(start..<end)
    }

    public var first: Optional[ArraySlice[T]] {
        if self.isEmpty { return .None }
        .Some(self(0))
    }

    public var last: Optional[ArraySlice[T]] {
        if self.isEmpty { return .None }
        .Some(self(self.count - 1))
    }

    public func iter() -> ChunksIterator[T] {
        ChunksIterator(slice: self.slice, offset: 0, chunkSize: self.chunkSize)
    }

    /// Copies every chunk into an owned array.
    public func toArray() -> Array[Array[T]] {
        var result = Array[Array[T]]();
        let n = self.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self(i).toArray())
        }
        result
    }
}

public struct ChunksIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var slice: ArraySlice[T]
    private var offset: Int64
    private var chunkSize: Int64

    public init(slice slice: ArraySlice[T], offset offset: Int64, chunkSize chunkSize: Int64) {
        self.slice = slice;
        self.offset = offset;
        self.chunkSize = chunkSize;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        let n = self.slice.count;
        if self.offset >= n {
            return .None
        }
        var end = self.offset + self.chunkSize;
        if end > n {
            end = n
        }
        let chunk = self.slice(self.offset..<end);
        self.offset = end;
        .Some(chunk)
    }
}

// ============================================================================
// WINDOWS VIEW
// ============================================================================

/// Multi-pass lazy view over overlapping fixed-size sliding windows.
public struct WindowsView[T]: Iterable {
    type Item = ArraySlice[T]
    type TargetIterator = WindowsIterator[T]

    private var slice: ArraySlice[T]
    private var windowSize: Int64

    public init(slice slice: ArraySlice[T], windowSize windowSize: Int64) {
        self.slice = slice;
        self.windowSize = windowSize;
    }

    public var count: Int64 {
        if self.slice.count < self.windowSize {
            return 0
        }
        self.slice.count - self.windowSize + 1
    }

    public var isEmpty: Bool { self.slice.count < self.windowSize }

    /// Window `index`. Panics when out of range.
    public subscript(index: Int64) -> ArraySlice[T] {
        if index < 0 or index >= self.count {
            fatalError("WindowsView index out of bounds")
        }
        self.slice(index..<(index + self.windowSize))
    }

    public var first: Optional[ArraySlice[T]] {
        if self.isEmpty { return .None }
        .Some(self(0))
    }

    public var last: Optional[ArraySlice[T]] {
        if self.isEmpty { return .None }
        .Some(self(self.count - 1))
    }

    public func iter() -> WindowsIterator[T] {
        WindowsIterator(slice: self.slice, offset: 0, windowSize: self.windowSize)
    }

    /// Copies every window into an owned array.
    public func toArray() -> Array[Array[T]] {
        var result = Array[Array[T]]();
        let n = self.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self(i).toArray())
        }
        result
    }
}

public struct WindowsIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var slice: ArraySlice[T]
    private var offset: Int64
    private var windowSize: Int64

    public init(slice slice: ArraySlice[T], offset offset: Int64, windowSize windowSize: Int64) {
        self.slice = slice;
        self.offset = offset;
        self.windowSize = windowSize;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        let end = self.offset + self.windowSize;
        if end > self.slice.count {
            return .None
        }
        let window = self.slice(self.offset..<end);
        self.offset = self.offset + 1;
        .Some(window)
    }
}

// ============================================================================
// REVERSED VIEW
// ============================================================================

/// Multi-pass lazy view that iterates back-to-front without allocating.
public struct ReversedView[T]: Iterable {
    type Item = T
    type TargetIterator = ReversedSliceIterator[T]

    private var slice: ArraySlice[T]

    public init(slice slice: ArraySlice[T]) {
        self.slice = slice;
    }

    public var count: Int64 { self.slice.count }

    public var isEmpty: Bool { self.slice.count == 0 }

    /// Element `index` counted from the back. Panics when out of range.
    public subscript(index: Int64) -> T {
        if index < 0 or index >= self.slice.count {
            fatalError("ReversedView index out of bounds")
        }
        self.slice(self.slice.count - 1 - index)
    }

    public var first: Optional[T] {
        if self.isEmpty { return .None }
        .Some(self.slice(self.slice.count - 1))
    }

    public var last: Optional[T] {
        if self.isEmpty { return .None }
        .Some(self.slice(0))
    }

    public func iter() -> ReversedSliceIterator[T] {
        ReversedSliceIterator(slice: self.slice, remaining: self.slice.count)
    }

    /// Copies the elements, back to front, into an owned array.
    public func toArray() -> Array[T] {
        var result = Array[T]();
        let n = self.slice.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self.slice(n - 1 - i))
        }
        result
    }
}

public struct ReversedSliceIterator[T]: Iterator {
    type Item = T

    private var slice: ArraySlice[T]
    private var remaining: Int64

    public init(slice slice: ArraySlice[T], remaining remaining: Int64) {
        self.slice = slice;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[T] {
        if self.remaining <= 0 {
            return .None
        }
        self.remaining = self.remaining - 1;
        .Some(self.slice(self.remaining))
    }
}

// ============================================================================
// ARRAY SPLIT VIEW (separator-based, T: Equatable)
// ============================================================================

/// Multi-pass lazy view over the segments produced by splitting on each
/// occurrence of a separator value. (Named `ArraySplitView` to avoid
/// collision with `std.text.SplitView`.)
public struct ArraySplitView[T]: Iterable where T: Equatable {
    type Item = ArraySlice[T]
    type TargetIterator = ArraySplitIterator[T]

    private var slice: ArraySlice[T]
    private var separator: T

    public init(slice slice: ArraySlice[T], separator separator: T) {
        self.slice = slice;
        self.separator = separator;
    }

    public var count: Int64 {
        var n: Int64 = 1;
        for i in 0..<self.slice.count {
            if self.slice(i).isEqual(to: self.separator) {
                n = n + 1
            }
        }
        n
    }

    public var isEmpty: Bool { false }

    public func iter() -> ArraySplitIterator[T] {
        ArraySplitIterator(slice: self.slice, offset: 0, separator: self.separator, done: false)
    }

    /// Copies every segment into an owned array.
    public func toArray() -> Array[Array[T]] {
        var result = Array[Array[T]]();
        for segment in self {
            result.append(segment.toArray())
        }
        result
    }
}

public struct ArraySplitIterator[T]: Iterator where T: Equatable {
    type Item = ArraySlice[T]

    private var slice: ArraySlice[T]
    private var offset: Int64
    private var separator: T
    private var done: Bool

    public init(slice slice: ArraySlice[T], offset offset: Int64, separator separator: T, done done: Bool) {
        self.slice = slice;
        self.offset = offset;
        self.separator = separator;
        self.done = done;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.done {
            return .None
        }
        let n = self.slice.count;
        var i = self.offset;
        while i < n {
            if self.slice(i).isEqual(to: self.separator) {
                let segment = self.slice(self.offset..<i);
                self.offset = i + 1;
                return .Some(segment)
            }
            i = i + 1
        }
        self.done = true;
        .Some(self.slice(self.offset..<n))
    }
}

// ============================================================================
// ARRAY SPLIT-WHERE VIEW (predicate-based)
// ============================================================================

/// Multi-pass lazy view over the segments produced by splitting on each
/// element matching a predicate. No `Equatable` requirement.
/// (Named `ArraySplitWhereView` to avoid collision with
/// `std.text.SplitWhereView`.)
public struct ArraySplitWhereView[T]: Iterable {
    type Item = ArraySlice[T]
    type TargetIterator = ArraySplitWhereIterator[T]

    private var slice: ArraySlice[T]
    private var predicate: escaping (T) -> Bool

    public init(slice slice: ArraySlice[T], consuming predicate predicate: escaping (T) -> Bool) {
        self.slice = slice;
        self.predicate = predicate;
    }

    public var count: Int64 {
        var n: Int64 = 1;
        for i in 0..<self.slice.count {
            if self.predicate(self.slice(i)) {
                n = n + 1
            }
        }
        n
    }

    public var isEmpty: Bool { false }

    public func iter() -> ArraySplitWhereIterator[T] {
        ArraySplitWhereIterator(slice: self.slice, offset: 0, predicate: self.predicate, done: false)
    }

    /// Copies every segment into an owned array.
    public func toArray() -> Array[Array[T]] {
        var result = Array[Array[T]]();
        for segment in self {
            result.append(segment.toArray())
        }
        result
    }
}

public struct ArraySplitWhereIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var slice: ArraySlice[T]
    private var offset: Int64
    private var predicate: escaping (T) -> Bool
    private var done: Bool

    public init(slice slice: ArraySlice[T], offset offset: Int64, consuming predicate predicate: escaping (T) -> Bool, done done: Bool) {
        self.slice = slice;
        self.offset = offset;
        self.predicate = predicate;
        self.done = done;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.done {
            return .None
        }
        let n = self.slice.count;
        var i = self.offset;
        while i < n {
            if self.predicate(self.slice(i)) {
                let segment = self.slice(self.offset..<i);
                self.offset = i + 1;
                return .Some(segment)
            }
            i = i + 1
        }
        self.done = true;
        .Some(self.slice(self.offset..<n))
    }
}
