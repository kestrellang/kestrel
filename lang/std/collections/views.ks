// Multi-pass lazy views over contiguous collections.

module std.collections

import std.core.(Bool, Equatable, fatalError)
import std.numeric.(Int64)
import std.result.(Optional)
import std.memory.(Pointer, ArraySlice)
import std.iter.(Iterator, Iterable)

// ============================================================================
// CHUNKS VIEW
// ============================================================================

/// Multi-pass lazy view over non-overlapping `chunkSize`-sized
/// `ArraySlice[T]` segments.
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
        let n = self.slice.count;
        (n + self.chunkSize - 1) / self.chunkSize
    }

    public var isEmpty: Bool { self.slice.count == 0 }

    public subscript(index: Int64) -> ArraySlice[T] {
        if index < 0 or index >= self.count {
            fatalError("ChunksView index out of bounds")
        }
        let start = index * self.chunkSize;
        let remaining = self.slice.count - start;
        var thisSize = self.chunkSize;
        if remaining < self.chunkSize {
            thisSize = remaining
        }
        ArraySlice(pointer: self.slice.pointer.offset(by: start), count: thisSize)
    }

    public var first: Optional[ArraySlice[T]] {
        if self.slice.count > 0 {
            .Some(self(0))
        } else {
            .None
        }
    }

    public var last: Optional[ArraySlice[T]] {
        if self.slice.count > 0 {
            .Some(self(self.count - 1))
        } else {
            .None
        }
    }

    public func iter() -> ChunksIterator[T] {
        ChunksIterator(ptr: self.slice.pointer, remaining: self.slice.count, chunkSize: self.chunkSize)
    }

    public func toArray() -> Array[ArraySlice[T]] {
        var result = Array[ArraySlice[T]]();
        let n = self.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self(i))
        }
        result
    }
}

public struct ChunksIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var chunkSize: Int64

    public init(ptr ptr: Pointer[T], remaining remaining: Int64, chunkSize chunkSize: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
        self.chunkSize = chunkSize;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.remaining <= 0 {
            return .None
        }
        var thisSize = self.chunkSize;
        if self.remaining < self.chunkSize {
            thisSize = self.remaining
        }
        let slice = ArraySlice(pointer: self.ptr, count: thisSize);
        self.ptr = self.ptr.offset(by: thisSize);
        self.remaining = self.remaining - thisSize;
        .Some(slice)
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

    public subscript(index: Int64) -> ArraySlice[T] {
        if index < 0 or index >= self.count {
            fatalError("WindowsView index out of bounds")
        }
        ArraySlice(pointer: self.slice.pointer.offset(by: index), count: self.windowSize)
    }

    public var first: Optional[ArraySlice[T]] {
        if self.isEmpty {
            return .None
        }
        .Some(self(0))
    }

    public var last: Optional[ArraySlice[T]] {
        if self.isEmpty {
            return .None
        }
        .Some(self(self.count - 1))
    }

    public func iter() -> WindowsIterator[T] {
        WindowsIterator(ptr: self.slice.pointer, totalCount: self.slice.count, windowSize: self.windowSize)
    }

    public func toArray() -> Array[ArraySlice[T]] {
        var result = Array[ArraySlice[T]]();
        let n = self.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self(i))
        }
        result
    }
}

public struct WindowsIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var windowSize: Int64

    public init(ptr ptr: Pointer[T], totalCount totalCount: Int64, windowSize windowSize: Int64) {
        self.ptr = ptr;
        self.windowSize = windowSize;
        let windowCount = totalCount - windowSize + 1;
        self.remaining = if windowCount > 0 {
            windowCount
        } else {
            0
        };
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.remaining <= 0 {
            return .None
        }
        let slice = ArraySlice(pointer: self.ptr, count: self.windowSize);
        self.ptr = self.ptr.offset(by: 1);
        self.remaining = self.remaining - 1;
        .Some(slice)
    }
}

// ============================================================================
// REVERSED VIEW
// ============================================================================

/// Multi-pass lazy view that iterates a contiguous collection
/// back-to-front without allocating.
public struct ReversedView[T]: Iterable {
    type Item = T
    type TargetIterator = ReversedSliceIterator[T]

    private var slice: ArraySlice[T]

    public init(slice slice: ArraySlice[T]) {
        self.slice = slice;
    }

    public var count: Int64 { self.slice.count }

    public var isEmpty: Bool { self.slice.count == 0 }

    public subscript(index: Int64) -> T {
        if index < 0 or index >= self.slice.count {
            fatalError("ReversedView index out of bounds")
        }
        self.slice.pointer.offset(by: self.slice.count - 1 - index).read()
    }

    public var first: Optional[T] {
        if self.slice.count > 0 {
            .Some(self.slice.pointer.offset(by: self.slice.count - 1).read())
        } else {
            .None
        }
    }

    public var last: Optional[T] {
        if self.slice.count > 0 {
            .Some(self.slice.pointer.read())
        } else {
            .None
        }
    }

    public func iter() -> ReversedSliceIterator[T] {
        ReversedSliceIterator(ptr: self.slice.pointer, remaining: self.slice.count)
    }

    public func toArray() -> Array[T] {
        var result = Array[T]();
        let n = self.slice.count;
        result.reserveCapacity(n);
        for i in 0..<n {
            result.append(self.slice.pointer.offset(by: n - 1 - i).read())
        }
        result
    }
}

public struct ReversedSliceIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    public mutating func next() -> Optional[T] {
        if self.remaining <= 0 {
            return .None
        }
        self.remaining = self.remaining - 1;
        .Some(self.ptr.offset(by: self.remaining).read())
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
        let p = self.slice.pointer;
        for i in 0..<self.slice.count {
            if p.offset(by: i).read().isEqual(to: self.separator) {
                n = n + 1
            }
        }
        n
    }

    public var isEmpty: Bool { false }

    public func iter() -> ArraySplitIterator[T] {
        ArraySplitIterator(ptr: self.slice.pointer, remaining: self.slice.count, separator: self.separator, done: false)
    }

    public func toArray() -> Array[ArraySlice[T]] {
        var result = Array[ArraySlice[T]]();
        let p = self.slice.pointer;
        var start: Int64 = 0;
        for i in 0..<self.slice.count {
            if p.offset(by: i).read().isEqual(to: self.separator) {
                result.append(ArraySlice(pointer: p.offset(by: start), count: i - start));
                start = i + 1
            }
        }
        result.append(ArraySlice(pointer: p.offset(by: start), count: self.slice.count - start));
        result
    }
}

public struct ArraySplitIterator[T]: Iterator where T: Equatable {
    type Item = ArraySlice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var separator: T
    private var done: Bool

    public init(ptr ptr: Pointer[T], remaining remaining: Int64, separator separator: T, done done: Bool) {
        self.ptr = ptr;
        self.remaining = remaining;
        self.separator = separator;
        self.done = done;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.done {
            return .None
        }
        var i: Int64 = 0;
        while i < self.remaining {
            if self.ptr.offset(by: i).read().isEqual(to: self.separator) {
                let segment = ArraySlice(pointer: self.ptr, count: i);
                self.ptr = self.ptr.offset(by: i + 1);
                self.remaining = self.remaining - i - 1;
                return .Some(segment)
            }
            i = i + 1
        }
        let segment = ArraySlice(pointer: self.ptr, count: self.remaining);
        self.done = true;
        .Some(segment)
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
    private var predicate: (T) -> Bool

    public init(slice slice: ArraySlice[T], predicate predicate: (T) -> Bool) {
        self.slice = slice;
        self.predicate = predicate;
    }

    public var count: Int64 {
        var n: Int64 = 1;
        let p = self.slice.pointer;
        for i in 0..<self.slice.count {
            if self.predicate(p.offset(by: i).read()) {
                n = n + 1
            }
        }
        n
    }

    public var isEmpty: Bool { false }

    public func iter() -> ArraySplitWhereIterator[T] {
        ArraySplitWhereIterator(ptr: self.slice.pointer, remaining: self.slice.count, predicate: self.predicate, done: false)
    }

    public func toArray() -> Array[ArraySlice[T]] {
        var result = Array[ArraySlice[T]]();
        let p = self.slice.pointer;
        var start: Int64 = 0;
        for i in 0..<self.slice.count {
            if self.predicate(p.offset(by: i).read()) {
                result.append(ArraySlice(pointer: p.offset(by: start), count: i - start));
                start = i + 1
            }
        }
        result.append(ArraySlice(pointer: p.offset(by: start), count: self.slice.count - start));
        result
    }
}

public struct ArraySplitWhereIterator[T]: Iterator {
    type Item = ArraySlice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var predicate: (T) -> Bool
    private var done: Bool

    public init(ptr ptr: Pointer[T], remaining remaining: Int64, predicate predicate: (T) -> Bool, done done: Bool) {
        self.ptr = ptr;
        self.remaining = remaining;
        self.predicate = predicate;
        self.done = done;
    }

    public mutating func next() -> Optional[ArraySlice[T]] {
        if self.done {
            return .None
        }
        var i: Int64 = 0;
        while i < self.remaining {
            if self.predicate(self.ptr.offset(by: i).read()) {
                let segment = ArraySlice(pointer: self.ptr, count: i);
                self.ptr = self.ptr.offset(by: i + 1);
                self.remaining = self.remaining - i - 1;
                return .Some(segment)
            }
            i = i + 1
        }
        let segment = ArraySlice(pointer: self.ptr, count: self.remaining);
        self.done = true;
        .Some(segment)
    }
}
