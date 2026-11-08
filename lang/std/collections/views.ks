// Multi-pass lazy views over a slice (and so over an array).
//
// Every view holds a `&Slice[T]`, so it borrows the elements: it can't
// outlive the slice it was made from. Like the slice itself, it is only
// valid until the underlying array next reallocates. The segments a view
// hands out are `&Slice[T]`s of the same elements. Their `toArray()`
// copies segments into owned arrays, since an array can't hold slices.

module std.collections

import std.core.(Bool, Equatable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.iter.(Iterator, Iterable)
import std.collections.(Array, Slice)

// ============================================================================
// CHUNKS VIEW
// ============================================================================

/// Multi-pass lazy view over non-overlapping `chunkSize`-sized segments.
/// The last chunk is shorter when the count isn't a multiple of the size.
public struct ChunksView[T]: Iterable {
    type Item = &Slice[T]
    type TargetIterator = ChunksIterator[T]

    fileprivate let slice: &Slice[T]
    fileprivate var chunkSize: Int64


    public var count: Int64 {
        (self.slice.count + self.chunkSize - 1) / self.chunkSize
    }

    public var isEmpty: Bool { self.slice.count == 0 }

    /// Chunk `index`. Panics when out of range.
    public subscript(index: Int64) -> Slice[T] {
        ref {
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
    }

    public var first: Optional[&Slice[T]] {
        if self.isEmpty { return .None }
        let r = &self(0);
        .Some(r)
    }

    public var last: Optional[&Slice[T]] {
        if self.isEmpty { return .None }
        let r = &self(self.count - 1);
        .Some(r)
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
    type Item = &Slice[T]

    fileprivate let slice: &Slice[T]
    fileprivate var offset: Int64
    fileprivate var chunkSize: Int64


    public mutating func next() -> Optional[&Slice[T]] {
        let n = self.slice.count;
        if self.offset >= n {
            return .None
        }
        var end = self.offset + self.chunkSize;
        if end > n {
            end = n
        }
        let chunk = &self.slice(self.offset..<end);
        self.offset = end;
        .Some(chunk)
    }
}

// ============================================================================
// WINDOWS VIEW
// ============================================================================

/// Multi-pass lazy view over overlapping fixed-size sliding windows.
public struct WindowsView[T]: Iterable {
    type Item = &Slice[T]
    type TargetIterator = WindowsIterator[T]

    fileprivate let slice: &Slice[T]
    fileprivate var windowSize: Int64


    public var count: Int64 {
        if self.slice.count < self.windowSize {
            return 0
        }
        self.slice.count - self.windowSize + 1
    }

    public var isEmpty: Bool { self.slice.count < self.windowSize }

    /// Window `index`. Panics when out of range.
    public subscript(index: Int64) -> Slice[T] {
        ref {
            if index < 0 or index >= self.count {
                fatalError("WindowsView index out of bounds")
            }
            self.slice(index..<(index + self.windowSize))
        }
    }

    public var first: Optional[&Slice[T]] {
        if self.isEmpty { return .None }
        let r = &self(0);
        .Some(r)
    }

    public var last: Optional[&Slice[T]] {
        if self.isEmpty { return .None }
        let r = &self(self.count - 1);
        .Some(r)
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
    type Item = &Slice[T]

    fileprivate let slice: &Slice[T]
    fileprivate var offset: Int64
    fileprivate var windowSize: Int64


    public mutating func next() -> Optional[&Slice[T]] {
        let end = self.offset + self.windowSize;
        if end > self.slice.count {
            return .None
        }
        let window = &self.slice(self.offset..<end);
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

    fileprivate let slice: &Slice[T]


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

    fileprivate let slice: &Slice[T]
    fileprivate var remaining: Int64


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
    type Item = &Slice[T]
    type TargetIterator = ArraySplitIterator[T]

    fileprivate let slice: &Slice[T]
    fileprivate var separator: T


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
    type Item = &Slice[T]

    fileprivate let slice: &Slice[T]
    fileprivate var offset: Int64
    fileprivate var separator: T
    fileprivate var done: Bool


    public mutating func next() -> Optional[&Slice[T]] {
        if self.done {
            return .None
        }
        let n = self.slice.count;
        var i = self.offset;
        while i < n {
            if self.slice(i).isEqual(to: self.separator) {
                let segment = &self.slice(self.offset..<i);
                self.offset = i + 1;
                return .Some(segment)
            }
            i = i + 1
        }
        self.done = true;
        let rest = &self.slice(self.offset..<n);
        .Some(rest)
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
    type Item = &Slice[T]
    type TargetIterator = ArraySplitWhereIterator[T]

    fileprivate let slice: &Slice[T]
    fileprivate var predicate: escaping (T) -> Bool


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
    type Item = &Slice[T]

    fileprivate let slice: &Slice[T]
    fileprivate var offset: Int64
    fileprivate var predicate: escaping (T) -> Bool
    fileprivate var done: Bool


    public mutating func next() -> Optional[&Slice[T]] {
        if self.done {
            return .None
        }
        let n = self.slice.count;
        var i = self.offset;
        while i < n {
            if self.predicate(self.slice(i)) {
                let segment = &self.slice(self.offset..<i);
                self.offset = i + 1;
                return .Some(segment)
            }
            i = i + 1
        }
        self.done = true;
        let rest = &self.slice(self.offset..<n);
        .Some(rest)
    }
}

// ============================================================================
// CONSTRUCTORS
// ============================================================================

extend Slice[T] where T: Copyable {
    /// Multi-pass lazy view over non-overlapping `size`-sized chunks. The
    /// last chunk is shorter when `count` isn't a multiple of `size`.
    /// Panics if `size <= 0`.
    ///
    /// ```
    /// let v = [1, 2, 3, 4, 5].chunks(of: 2);
    /// v.count;          // 3
    /// v(2).count;       // 1
    /// ```
    public func chunks(of size: Int64) -> ChunksView[T] {
        if size <= 0 {
            fatalError("chunks: size must be positive")
        }
        let s = &self;
        ChunksView(slice: s, chunkSize: size)
    }

    /// Multi-pass lazy view over overlapping `size`-sized sliding windows.
    /// Empty when there are fewer than `size` elements. Panics if
    /// `size <= 0`.
    ///
    /// ```
    /// [1, 2, 3, 4].windows(of: 2).count;   // 3
    /// ```
    public func windows(of size: Int64) -> WindowsView[T] {
        if size <= 0 {
            fatalError("windows: size must be positive")
        }
        let s = &self;
        WindowsView(slice: s, windowSize: size)
    }

    /// Multi-pass lazy view that reads back to front.
    ///
    /// ```
    /// let v = [1, 2, 3].reversed();
    /// v(0);           // 3
    /// v.toArray();    // [3, 2, 1]
    /// ```
    public func reversed() -> ReversedView[T] {
        let s = &self;
        ReversedView(slice: s)
    }

    /// Multi-pass lazy view over the segments between elements matching
    /// `predicate`. The matching elements are dropped.
    public func split(consuming where predicate: escaping (T) -> Bool) -> ArraySplitWhereView[T] {
        let s = &self;
        ArraySplitWhereView(slice: s, predicate: predicate)
    }
}

extend Slice[T] where T: Equatable, T: Copyable {
    /// Multi-pass lazy view over the segments between occurrences of
    /// `separator`. `toArray()` copies them into owned arrays.
    ///
    /// ```
    /// [1, 0, 2, 0, 3].split(0).toArray();   // [[1], [2], [3]]
    /// ```
    public func split(separator: T) -> ArraySplitView[T] {
        let s = &self;
        ArraySplitView(slice: s, separator: separator)
    }
}

extend Array[T] {
    /// See `Slice.chunks(of:)`.
    public func chunks(of size: Int64) -> ChunksView[T] { self.asSlice().chunks(of: size) }
    /// See `Slice.windows(of:)`.
    public func windows(of size: Int64) -> WindowsView[T] { self.asSlice().windows(of: size) }
    /// See `Slice.reversed()`.
    public func reversed() -> ReversedView[T] { self.asSlice().reversed() }
    /// See `Slice.split(where:)`.
    public func split(consuming where predicate: escaping (T) -> Bool) -> ArraySplitWhereView[T] {
        let s = &self.asSlice();
        ArraySplitWhereView(slice: s, predicate: predicate)
    }
}

extend Array[T] where T: Equatable {
    /// See `Slice.split(separator:)`.
    public func split(separator: T) -> ArraySplitView[T] {
        let s = &self.asSlice();
        ArraySplitView(slice: s, separator: separator)
    }
}
