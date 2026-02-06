// Array[T] - dynamic growable array with COW (Copy-on-Write) semantics

module std.collections

import std.core.(Bool, Equatable, Comparable, Cloneable, ArrayMatchable, Defaultable)
import std.core.(ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral)
import std.core.(Range, Hash)
import std.text.(Formattable, FormatOptions)
import std.num.(Int64)
import std.num.(RandomNumberGenerator, Lcg64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, Slice, RawPointer, SystemAllocator, LiteralSlice, RcBox)
import std.iter.(Iterator, Iterable)
import std.text.(String)

// ============================================================================
// ARRAY ITERATOR
// ============================================================================

/// Iterator over array elements.
public struct ArrayIterator[T]: Iterator {
    type Item = T

    private var ptr: Pointer[T]
    private var remaining: Int64

    /// Creates an array iterator.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
    }

    /// Returns the next element, or None if exhausted.
    public mutating func next() -> T? {
        if self.remaining > Int64(intLiteral: 0) {
            let value = self.ptr.read();
            self.ptr = self.ptr.offset(by: Int64(intLiteral: 1));
            self.remaining = self.remaining - Int64(intLiteral: 1);
            .Some(value)
        } else {
            .None
        }
    }
}

// ============================================================================
// CHUNKS ITERATOR
// ============================================================================

/// Iterator over non-overlapping chunks of an array.
///
/// The last chunk may be smaller than the chunk size if the array
/// length is not evenly divisible.
///
/// Example:
///     let arr = [1, 2, 3, 4, 5]
///     for chunk in arr.chunks(of: 2) {
///         // yields: [1, 2], then [3, 4], then [5]
///     }
public struct ChunksIterator[T]: Iterator {
    type Item = Slice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var chunkSize: Int64

    /// Creates a chunks iterator.
    public init(ptr ptr: Pointer[T], remaining remaining: Int64, chunkSize chunkSize: Int64) {
        self.ptr = ptr;
        self.remaining = remaining;
        self.chunkSize = chunkSize;
    }

    /// Returns the next chunk, or None if exhausted.
    public mutating func next() -> Slice[T]? {
        if self.remaining <= Int64(intLiteral: 0) {
            return .None
        }

        // Determine this chunk's actual size (may be smaller for last chunk)
        let thisChunkSize: Int64 = if self.remaining < self.chunkSize {
            self.remaining
        } else {
            self.chunkSize
        };

        let slice = Slice(pointer: self.ptr, count: thisChunkSize);
        self.ptr = self.ptr.offset(by: thisChunkSize);
        self.remaining = self.remaining - thisChunkSize;
        .Some(slice)
    }
}

// ============================================================================
// WINDOWS ITERATOR
// ============================================================================

/// Iterator over overlapping sliding windows of an array.
///
/// All windows have exactly the specified size. If the array is
/// smaller than the window size, no windows are yielded.
///
/// Example:
///     let arr = [1, 2, 3, 4]
///     for window in arr.windows(of: 2) {
///         // yields: [1, 2], then [2, 3], then [3, 4]
///     }
public struct WindowsIterator[T]: Iterator {
    type Item = Slice[T]

    private var ptr: Pointer[T]
    private var remaining: Int64
    private var windowSize: Int64

    /// Creates a windows iterator.
    public init(ptr ptr: Pointer[T], totalCount totalCount: Int64, windowSize windowSize: Int64) {
        self.ptr = ptr;
        self.windowSize = windowSize;
        // Number of windows = totalCount - windowSize + 1 (if positive)
        let windowCount = totalCount - windowSize + Int64(intLiteral: 1);
        self.remaining = if windowCount > Int64(intLiteral: 0) {
            windowCount
        } else {
            Int64(intLiteral: 0)
        };
    }

    /// Returns the next window, or None if exhausted.
    public mutating func next() -> Slice[T]? {
        if self.remaining <= Int64(intLiteral: 0) {
            return .None
        }

        let slice = Slice(pointer: self.ptr, count: self.windowSize);
        self.ptr = self.ptr.offset(by: Int64(intLiteral: 1));
        self.remaining = self.remaining - Int64(intLiteral: 1);
        .Some(slice)
    }
}

// ============================================================================
// ARRAY STORAGE (Internal)
// ============================================================================

/// Internal storage for Array (ptr, len, cap).
struct ArrayStorage[T]: Cloneable {
    var ptr: Pointer[T]
    var len: Int64
    var cap: Int64

    init(ptr ptr: Pointer[T], len len: Int64, cap cap: Int64) {
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }

    /// Deep clone - allocate new buffer and copy elements.
    func clone() -> ArrayStorage[T] {
        if self.len == Int64(intLiteral: 0) {
            return ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            )
        }
        let layout = Layout.array[T](self.len);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            // Copy elements
            for i in 0..<self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read());
            }
            ArrayStorage(ptr: newPtr, len: self.len, cap: self.len)
        } else {
            lang.panic("ArrayStorage clone allocation failed")
        }
    }

    deinit {
        if self.cap > Int64(intLiteral: 0) {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// ============================================================================
// ARRAY
// ============================================================================

/// A dynamic growable array with copy-on-write semantics.
///
/// Arrays automatically grow when elements are added and use COW
/// to efficiently share data between copies until mutation occurs.
@builtin(.ArrayStruct)
public struct Array[T]: Iterable, ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral, Cloneable, Defaultable {
    type Item = T
    type Iter = ArrayIterator[T]
    type Element = T

    fileprivate var storage: RcBox[ArrayStorage[T]]

    // Helper accessors for storage fields
    fileprivate func ptr() -> Pointer[T] { self.storage.getValue().ptr }
    fileprivate func len() -> Int64 { self.storage.getValue().len }
    fileprivate func cap() -> Int64 { self.storage.getValue().cap }

    // Ensure unique storage for mutation (COW)
    fileprivate mutating func makeUnique() {
        if self.storage.isUnique() == false {
            self.storage = self.storage.deepClone()
        }
    }

    /// Private init for internal use (from storage).
    private init(storage storage: RcBox[ArrayStorage[T]]) {
        self.storage = storage;
    }

    // ========================================================================
    // CONSTRUCTORS
    // ========================================================================

    /// Creates an empty array.
    public init() {
        self.storage = RcBox(ArrayStorage(
            ptr: Pointer(raw: lang.ptr_null[T]()),
            len: Int64(intLiteral: 0),
            cap: Int64(intLiteral: 0)
        ));
    }

    /// Creates an empty array with the specified capacity.
    public init(capacity capacity: Int64) {
        if capacity > Int64(intLiteral: 0) {
            let layout = Layout.array[T](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                self.storage = RcBox(ArrayStorage(
                    ptr: rawPtr.cast[T](),
                    len: Int64(intLiteral: 0),
                    cap: capacity
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    /// Internal initializer called by compiler for array literals.
    public init(_arrayLiteralPointer: lang.ptr[T], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount))
    }

    /// Creates an array from an array literal.
    public init(arrayLiteral elements: LiteralSlice[T]) {
        let elementCount = elements.count();
        if elementCount > Int64(intLiteral: 0) {
            let layout = Layout.array[T](elementCount);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                var currentLen: Int64 = Int64(intLiteral: 0);
                // Copy elements from literal slice
                var iter = elements.iter();
                while let .Some(item) = iter.next() {
                    newPtr.offset(by: currentLen).write(item);
                    currentLen = currentLen + Int64(intLiteral: 1)
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: currentLen,
                    cap: elementCount
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        } else {
            self.storage = RcBox(ArrayStorage(
                ptr: Pointer(raw: lang.ptr_null[T]()),
                len: Int64(intLiteral: 0),
                cap: Int64(intLiteral: 0)
            ))
        }
    }

    /// Creates an array by repeating a value count times.
    ///
    /// Example:
    ///     let zeros = Array(repeating: 0, count: 5)  // [0, 0, 0, 0, 0]
    ///     let empty = Array(repeating: "x", count: 0)  // []
    public init(repeating value: T, count: Int64) {
        if count <= Int64(intLiteral: 0) {
            self.init()
        } else {
            let layout = Layout.array[T](count);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                // Write first element directly
                newPtr.write(value);
                // Copy for remaining elements
                for i in 1..<count {
                    newPtr.offset(by: i).write(value);
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        }
    }

    /// Creates an array from any iterable source.
    ///
    /// Example:
    ///     let fromRange = Array(from: 1..<5)  // [1, 2, 3, 4]
    ///     let fromSet = Array(from: mySet)
    public init[I](from iterable: I) where I: Iterable, I.Item = T {
        self.init();
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    /// Creates an array of count elements using a generator function.
    ///
    /// The generator receives the index (0 to count-1) for each element.
    ///
    /// Example:
    ///     let squares = Array(count: 5, generator: { (i) in i * i })  // [0, 1, 4, 9, 16]
    ///     let indices = Array(count: 3, generator: { (i) in i })  // [0, 1, 2]
    public init(count: Int64, generator: (Int64) -> T) {
        if count <= Int64(intLiteral: 0) {
            self.init()
        } else {
            let layout = Layout.array[T](count);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                let newPtr = rawPtr.cast[T]();
                for i in 0..<count {
                    newPtr.offset(by: i).write(generator(i));
                }
                self.storage = RcBox(ArrayStorage(
                    ptr: newPtr,
                    len: count,
                    cap: count
                ))
            } else {
                lang.panic("Array allocation failed")
            }
        }
    }

    // ========================================================================
    // PROPERTIES
    // ========================================================================

    /// Returns the number of elements in the array.
    ///
    /// Example:
    ///     [1, 2, 3].count  // 3
    ///     [].count         // 0
    public var count: Int64 { get { self.len() } }

    /// Returns the current capacity (elements storable without reallocating).
    ///
    /// Capacity is always >= count. When count exceeds capacity, the array
    /// reallocates with increased capacity (typically doubling).
    ///
    /// Example:
    ///     var arr = Array[Int64](capacity: 10)
    ///     arr.capacity  // >= 10
    public var capacity: Int64 { self.cap() }

    /// Returns true if the array contains no elements.
    ///
    /// Equivalent to `count == 0` but may be more readable.
    ///
    /// Example:
    ///     [].isEmpty      // true
    ///     [1].isEmpty     // false
    public var isEmpty: Bool { self.len() == Int64(intLiteral: 0) }

    /// Returns the valid index range for this array.
    ///
    /// Equivalent to `0..<count`. Useful for iteration or bounds checking.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr.indices  // 0..<3
    ///
    ///     for i in arr.indices {
    ///         print(arr(i))
    ///     }
    public var indices: Range[Int64] {
        Range(Int64(intLiteral: 0), self.len())
    }

    // ========================================================================
    // ACCESSORS
    // ========================================================================

    /// Returns a raw pointer to the array's element storage.
    ///
    /// WARNING: The pointer is invalidated by any mutation or reallocation.
    /// Use with caution for FFI or low-level operations.
    public func asPointer() -> Pointer[T] { self.ptr() }

    /// Returns a slice view of the entire array.
    ///
    /// Example:
    ///     let slice = arr.asSlice()
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr(), count: self.len())
    }

    /// Returns true if index is within valid bounds [0, count).
    ///
    /// Example:
    ///     let arr = [1, 2, 3]
    ///     arr.isValidIndex(index: 0)   // true
    ///     arr.isValidIndex(index: 2)   // true
    ///     arr.isValidIndex(index: 3)   // false
    ///     arr.isValidIndex(index: -1)  // false
    public func isValidIndex(index: Int64) -> Bool {
        index >= Int64(intLiteral: 0) and index < self.len()
    }

    // ========================================================================
    // ELEMENT SUBSCRIPTS
    // ========================================================================

    /// Accesses the element at the given index.
    ///
    /// Panics if index is out of bounds [0, count).
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(0)      // 10
    ///     arr(1) = 25 // arr is now [10, 25, 30]
    ///     arr(5)      // PANIC: index out of bounds
    public subscript(index: Int64) -> T {
        get {
            if index < Int64(intLiteral: 0) or index >= self.len() {
                lang.panic("Array index out of bounds")
            }
            self.ptr().offset(by: index).read()
        }
        set {
            if index < Int64(intLiteral: 0) or index >= self.len() {
                lang.panic("Array index out of bounds")
            }
            self.makeUnique();
            self.ptr().offset(by: index).write(newValue)
        }
    }

    /// Accesses the element at the given index with bounds checking.
    ///
    /// Returns None if index is out of bounds, making it safe for
    /// untrusted indices.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(checked: 0)   // Some(10)
    ///     arr(checked: 5)   // None
    ///     arr(checked: -1)  // None
    public subscript(checked index: Int64) -> T? {
        get {
            if index >= Int64(intLiteral: 0) and index < self.len() {
                .Some(self.ptr().offset(by: index).read())
            } else {
                .None
            }
        }
    }

    /// Accesses the element at the given index without bounds checking.
    ///
    /// WARNING: Undefined behavior if index is out of bounds.
    /// Only use when you have already verified the index is valid.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     if arr.isValidIndex(index: i) {
    ///         let val = arr(unchecked: i)  // safe
    ///     }
    public subscript(unchecked index: Int64) -> T {
        get { self.ptr().offset(by: index).read() }
        set {
            self.makeUnique();
            self.ptr().offset(by: index).write(newValue)
        }
    }

    /// Accesses the element with wrapping for negative and overflow indices.
    ///
    /// Index -1 refers to the last element, -2 to second-to-last, etc.
    /// Positive indices beyond count also wrap using modulo arithmetic.
    /// Returns None only if the array is empty.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(wrapping: -1)  // Some(30) - last element
    ///     arr(wrapping: -2)  // Some(20) - second to last
    ///     arr(wrapping: 3)   // Some(10) - wraps to index 0
    ///     arr(wrapping: 4)   // Some(20) - wraps to index 1
    ///     [](wrapping: 0)    // None - empty array
    public subscript(wrapping index: Int64) -> T? {
        get {
            let myLen = self.len();
            if myLen == Int64(intLiteral: 0) {
                return .None
            }
            var idx = index % myLen;
            if idx < Int64(intLiteral: 0) {
                idx = idx + myLen
            }
            .Some(self.ptr().offset(by: idx).read())
        }
        set {
            if let .Some(value) = newValue {
                let myLen = self.len();
                if myLen == Int64(intLiteral: 0) {
                    return
                }
                var idx = index % myLen;
                if idx < Int64(intLiteral: 0) {
                    idx = idx + myLen
                }
                self.makeUnique();
                self.ptr().offset(by: idx).write(value)
            }
        }
    }

    /// Accesses element at index, clamping to valid bounds.
    ///
    /// Negative indices clamp to 0, indices >= count clamp to count-1.
    /// Returns None only if the array is empty.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(clamping: -5)   // Some(10) - clamped to first
    ///     arr(clamping: 100)  // Some(30) - clamped to last
    ///     arr(clamping: 1)    // Some(20) - normal access
    ///     [](clamping: 0)     // None - empty array
    public subscript(clamping index: Int64) -> T? {
        get {
            let myLen = self.len();
            if myLen == Int64(intLiteral: 0) {
                return .None
            }
            var idx = index;
            if idx < Int64(intLiteral: 0) {
                idx = Int64(intLiteral: 0)
            }
            if idx >= myLen {
                idx = myLen - Int64(intLiteral: 1)
            }
            .Some(self.ptr().offset(by: idx).read())
        }
        set {
            if let .Some(value) = newValue {
                let myLen = self.len();
                if myLen == Int64(intLiteral: 0) {
                    return
                }
                var idx = index;
                if idx < Int64(intLiteral: 0) {
                    idx = Int64(intLiteral: 0)
                }
                if idx >= myLen {
                    idx = myLen - Int64(intLiteral: 1)
                }
                self.makeUnique();
                self.ptr().offset(by: idx).write(value)
            }
        }
    }

    // ========================================================================
    // RANGE SUBSCRIPTS
    // ========================================================================

    /// Returns a slice view of the array for the given range.
    ///
    /// Panics if range is out of bounds.
    ///
    /// Example:
    ///     let arr = [10, 20, 30, 40, 50]
    ///     arr(1..<4)  // Slice containing [20, 30, 40]
    ///     arr(0..<2)  // Slice containing [10, 20]
    public subscript(range range: Range[Int64]) -> Slice[T] {
        get {
            let start = range.start;
            let end = range.end;
            if start < Int64(intLiteral: 0) or end > self.len() or start > end {
                lang.panic("Array range out of bounds")
            }
            Slice(pointer: self.ptr().offset(by: start), count: end - start)
        }
    }

    /// Returns a slice view with bounds checking.
    ///
    /// Returns None if any part of range is out of bounds.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(checked: 0..<2)   // Some(Slice[10, 20])
    ///     arr(checked: 0..<10)  // None - end out of bounds
    ///     arr(checked: -1..<2)  // None - start out of bounds
    public subscript(checkedRange range: Range[Int64]) -> Slice[T]? {
        get {
            let start = range.start;
            let end = range.end;
            if start >= Int64(intLiteral: 0) and end <= self.len() and start <= end {
                .Some(Slice(pointer: self.ptr().offset(by: start), count: end - start))
            } else {
                .None
            }
        }
    }

    /// Returns a slice view without bounds checking.
    ///
    /// WARNING: Undefined behavior if range is out of bounds.
    ///
    /// Example:
    ///     let arr = [10, 20, 30, 40, 50]
    ///     if start >= 0 and end <= arr.count {
    ///         let slice = arr(unchecked: start..<end)  // safe
    ///     }
    public subscript(uncheckedRange range: Range[Int64]) -> Slice[T] {
        get {
            Slice(pointer: self.ptr().offset(by: range.start), count: range.end - range.start)
        }
    }

    /// Returns a slice view with indices clamped to valid bounds.
    ///
    /// Never panics - out-of-bounds indices are clamped to [0, count].
    /// An empty slice is returned if the clamped range is empty.
    ///
    /// Example:
    ///     let arr = [10, 20, 30]
    ///     arr(clamping: -5..<100)  // Slice containing entire array
    ///     arr(clamping: -5..<1)    // Slice containing [10]
    ///     arr(clamping: 10..<20)   // Empty slice (both clamped to 3)
    public subscript(clampingRange range: Range[Int64]) -> Slice[T] {
        get {
            let myLen = self.len();
            var start = range.start;
            var end = range.end;
            if start < Int64(intLiteral: 0) { start = Int64(intLiteral: 0) }
            if end > myLen { end = myLen }
            if start > end { start = end }
            Slice(pointer: self.ptr().offset(by: start), count: end - start)
        }
    }

    // ========================================================================
    // CAPACITY MANAGEMENT (Internal)
    // ========================================================================

    /// Grows capacity to at least minCapacity.
    private mutating func grow(minCapacity: Int64) {
        let myCap = self.cap();
        if myCap >= minCapacity {
            return
        }

        self.makeUnique();

        // Calculate new capacity
        var newCap: Int64 = myCap;
        if newCap == Int64(intLiteral: 0) {
            newCap = Int64(intLiteral: 4)
        }
        while newCap < minCapacity {
            newCap = newCap * Int64(intLiteral: 2)
        }

        // Allocate new buffer
        let newLayout = Layout.array[T](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            let oldStorage = self.storage.getValue();
            // Copy existing elements
            for i in 0..<oldStorage.len {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read());
            }
            // Free old buffer
            if oldStorage.cap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[T](oldStorage.cap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(ArrayStorage(ptr: newPtr, len: oldStorage.len, cap: newCap))
        } else {
            lang.panic("Array grow failed")
        }
    }

    // ========================================================================
    // ELEMENT ACCESS
    // ========================================================================

    /// Sets the element at the given index without bounds checking.
    public mutating func setUnchecked(index: Int64, value: T) {
        self.makeUnique();
        self.ptr().offset(by: index).write(value)
    }

    // ========================================================================
    // ADDING ELEMENTS
    // ========================================================================

    /// Adds an element to the end of the array.
    ///
    /// Amortized O(1). May trigger reallocation if capacity is exceeded.
    ///
    /// Example:
    ///     var arr = [1, 2]
    ///     arr.append( 3)  // [1, 2, 3]
    public mutating func append(element: T) {
        let myLen = self.len();
        self.makeUnique();
        self.grow(myLen + Int64(intLiteral: 1));
        var s = self.storage.getValue();
        s.ptr.offset(by: s.len).write(element);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    /// Appends all elements from another array.
    ///
    /// Example:
    ///     var arr = [1, 2]
    ///     arr.append(contentsOf: [3, 4])  // [1, 2, 3, 4]
    public mutating func append(contentsOf other: Array[T]) {
        let otherLen = other.count;
        if otherLen == Int64(intLiteral: 0) {
            return
        }
        let myLen = self.len();
        self.makeUnique();
        self.grow(myLen + otherLen);
        var s = self.storage.getValue();
        let otherPtr = other.asPointer();
        for i in 0..<otherLen {
            s.ptr.offset(by: s.len).write(otherPtr.offset(by: i).read());
            s.len = s.len + Int64(intLiteral: 1)
        }
        self.storage.setValue(s)
    }

    /// Appends all elements from an iterable source.
    ///
    /// Example:
    ///     var arr = [1, 2]
    ///     arr.appendFrom(3..<6)  // [1, 2, 3, 4, 5]
    public mutating func appendFrom[I](iterable: I) where I: Iterable, I.Item = T {
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    /// Inserts an element at the specified index.
    ///
    /// Shifts all elements from index onward to the right.
    /// Panics if index > count (index == count appends to end).
    ///
    /// Example:
    ///     var arr = [1, 3]
    ///     arr.insert(element: 2, at: 1)  // [1, 2, 3]
    ///     arr.insert(element: 0, at: 0)  // [0, 1, 2, 3]
    ///     arr.insert(element: 4, at: 4)  // [0, 1, 2, 3, 4] - append
    public mutating func insert(element: T, at index: Int64) {
        let myLen = self.len();
        if index < Int64(intLiteral: 0) or index > myLen {
            lang.panic("Array.insert: index out of bounds")
        }
        self.makeUnique();
        self.grow(myLen + Int64(intLiteral: 1));
        var s = self.storage.getValue();
        // Shift elements right
        var i: Int64 = s.len;
        while i > index {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i - Int64(intLiteral: 1)).read());
            i = i - Int64(intLiteral: 1)
        }
        s.ptr.offset(by: index).write(element);
        s.len = s.len + Int64(intLiteral: 1);
        self.storage.setValue(s)
    }

    // ========================================================================
    // REMOVING ELEMENTS
    // ========================================================================

    /// Removes and returns the last element, or None if empty.
    ///
    /// Example:
    ///     var arr = [1, 2, 3]
    ///     arr.pop()  // Some(3), arr is [1, 2]
    ///     arr.pop()  // Some(2), arr is [1]
    ///     arr.pop()  // Some(1), arr is []
    ///     arr.pop()  // None, arr is still []
    public mutating func pop() -> T? {
        let myLen = self.len();
        if myLen > Int64(intLiteral: 0) {
            self.makeUnique();
            var s = self.storage.getValue();
            s.len = s.len - Int64(intLiteral: 1);
            let value = s.ptr.offset(by: s.len).read();
            self.storage.setValue(s);
            .Some(value)
        } else {
            .None
        }
    }

    /// Removes and returns the first element, or None if empty.
    ///
    /// Note: O(n) as all elements must shift left.
    ///
    /// Example:
    ///     var arr = [1, 2, 3]
    ///     arr.popFirst()  // Some(1), arr is [2, 3]
    public mutating func popFirst() -> T? {
        if self.len() == Int64(intLiteral: 0) {
            return .None
        }
        .Some(self.remove(at: Int64(intLiteral: 0)))
    }

    /// Removes and returns the element at the specified index.
    ///
    /// Shifts subsequent elements left. Panics if index is out of bounds.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4]
    ///     arr.remove(at: 1)  // returns 2, arr is [1, 3, 4]
    public mutating func remove(at index: Int64) -> T {
        let myLen = self.len();
        if index < Int64(intLiteral: 0) or index >= myLen {
            lang.panic("Array.remove: index out of bounds")
        }
        self.makeUnique();
        var s = self.storage.getValue();
        let removed = s.ptr.offset(by: index).read();
        // Shift elements left
        var i: Int64 = index;
        while i < s.len - Int64(intLiteral: 1) {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i + Int64(intLiteral: 1)).read());
            i = i + Int64(intLiteral: 1)
        }
        s.len = s.len - Int64(intLiteral: 1);
        self.storage.setValue(s);
        removed
    }

    /// Removes elements in the specified range.
    ///
    /// Panics if range is out of bounds.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.removeSubrange(range: 1..<4)  // arr is [1, 5]
    public mutating func removeSubrange(range: Range[Int64]) {
        let start = range.start;
        let end = range.end;
        let myLen = self.len();
        if start < Int64(intLiteral: 0) or end > myLen or start > end {
            lang.panic("Array.removeSubrange: range out of bounds")
        }
        let removeCount = end - start;
        if removeCount == Int64(intLiteral: 0) {
            return
        }
        self.makeUnique();
        var s = self.storage.getValue();
        // Shift elements left
        var i = start;
        while i < myLen - removeCount {
            s.ptr.offset(by: i).write(s.ptr.offset(by: i + removeCount).read());
            i = i + Int64(intLiteral: 1)
        }
        s.len = s.len - removeCount;
        self.storage.setValue(s)
    }

    /// Removes all elements from the array.
    ///
    /// Capacity may be retained for reuse.
    ///
    /// Example:
    ///     var arr = [1, 2, 3]
    ///     arr.clear()  // arr is []
    public mutating func clear() {
        self.makeUnique();
        var s = self.storage.getValue();
        s.len = Int64(intLiteral: 0);
        self.storage.setValue(s)
    }

    /// Retains only elements that satisfy the predicate.
    ///
    /// Elements are visited in order. This is an in-place filter.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.retain(matching: { (x) in x % 2 == 0 })  // arr is [2, 4]
    public mutating func retain(matching predicate: (T) -> Bool) {
        self.makeUnique();
        var s = self.storage.getValue();
        var writeIdx: Int64 = Int64(intLiteral: 0);
        for readIdx in 0..<s.len {
            let element = s.ptr.offset(by: readIdx).read();
            if predicate(element) {
                if writeIdx != readIdx {
                    s.ptr.offset(by: writeIdx).write(element)
                }
                writeIdx = writeIdx + Int64(intLiteral: 1)
            }
        }
        s.len = writeIdx;
        self.storage.setValue(s)
    }

    /// Removes all elements that satisfy the predicate.
    ///
    /// The inverse of `retain`. Elements are visited in order.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.removeAll(matching: { (x) in x % 2 == 0 })  // arr is [1, 3, 5]
    ///
    ///     var names = ["Alice", "", "Bob", ""]
    ///     names.removeAll(matching: |s| s.isEmpty)  // ["Alice", "Bob"]
    public mutating func removeAll(matching predicate: (T) -> Bool) {
        self.retain(matching: { (x) in predicate(x) == false })
    }

    // ========================================================================
    // REORDERING
    // ========================================================================

    /// Swaps the elements at the two given indices.
    ///
    /// Panics if either index is out of bounds.
    ///
    /// Example:
    ///     var arr = [1, 2, 3]
    ///     arr.swap(at: 0, with: 2)  // [3, 2, 1]
    public mutating func swap(at i: Int64, with j: Int64) {
        let myLen = self.len();
        if i < Int64(intLiteral: 0) or i >= myLen or j < Int64(intLiteral: 0) or j >= myLen {
            lang.panic("Array.swap: index out of bounds")
        }
        if i == j {
            return
        }
        self.makeUnique();
        let ptr = self.ptr();
        let temp = ptr.offset(by: i).read();
        ptr.offset(by: i).write(ptr.offset(by: j).read());
        ptr.offset(by: j).write(temp)
    }

    /// Reverses the order of elements in place.
    ///
    /// Example:
    ///     var arr = [1, 2, 3]
    ///     arr.reverse()  // [3, 2, 1]
    public mutating func reverse() {
        self.makeUnique();
        var s = self.storage.getValue();
        var left: Int64 = Int64(intLiteral: 0);
        var right: Int64 = s.len - Int64(intLiteral: 1);
        while left < right {
            let temp = s.ptr.offset(by: left).read();
            s.ptr.offset(by: left).write(s.ptr.offset(by: right).read());
            s.ptr.offset(by: right).write(temp);
            left = left + Int64(intLiteral: 1);
            right = right - Int64(intLiteral: 1)
        }
        self.storage.setValue(s)
    }

    /// Returns a new array with elements in reversed order.
    ///
    /// The original array is unchanged.
    ///
    /// Example:
    ///     let arr = [1, 2, 3]
    ///     let rev = arr.reversed()  // [3, 2, 1]
    ///     // arr is still [1, 2, 3]
    public func reversed() -> Array[T] {
        var result = self.clone();
        result.reverse();
        result
    }

    /// Rotates elements left by the given amount.
    ///
    /// Positive amounts rotate left (first elements move to end).
    /// Negative amounts rotate right (last elements move to start).
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.rotate(by: 2)   // [3, 4, 5, 1, 2]
    ///     arr.rotate(by: -1)  // [2, 3, 4, 5, 1]
    public mutating func rotate(by amount: Int64) {
        let myLen = self.len();
        if myLen <= Int64(intLiteral: 1) {
            return
        }
        var normalized = amount % myLen;
        if normalized < Int64(intLiteral: 0) {
            normalized = normalized + myLen
        }
        if normalized == Int64(intLiteral: 0) {
            return
        }
        // Three-reversal algorithm
        self.makeUnique();
        // Reverse first part [0, normalized)
        self.reverseRange(from: Int64(intLiteral: 0), to: normalized);
        // Reverse second part [normalized, len)
        self.reverseRange(from: normalized, to: myLen);
        // Reverse entire array
        self.reverse()
    }

    /// Private helper to reverse a range within the array.
    private mutating func reverseRange(from start: Int64, to end: Int64) {
        var left = start;
        var right = end - Int64(intLiteral: 1);
        let ptr = self.ptr();
        while left < right {
            let temp = ptr.offset(by: left).read();
            ptr.offset(by: left).write(ptr.offset(by: right).read());
            ptr.offset(by: right).write(temp);
            left = left + Int64(intLiteral: 1);
            right = right - Int64(intLiteral: 1)
        }
    }

    /// Replaces elements in the specified range with elements from replacement.
    ///
    /// The replacement can have a different length than the range.
    /// Panics if range is out of bounds.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.replaceSubrange(range: 1..<4, with: [20, 30])  // [1, 20, 30, 5]
    public mutating func replaceSubrange(range: Range[Int64], with replacement: Array[T]) {
        let start = range.start;
        let end = range.end;
        let myLen = self.len();
        if start < Int64(intLiteral: 0) or end > myLen or start > end {
            lang.panic("Array.replaceSubrange: range out of bounds")
        }

        let removeCount = end - start;
        let insertCount = replacement.count;
        let newLen = myLen - removeCount + insertCount;

        self.grow(newLen);
        self.makeUnique();
        var s = self.storage.getValue();

        if insertCount > removeCount {
            // Shift elements right
            var i = myLen - Int64(intLiteral: 1);
            while i >= end {
                s.ptr.offset(by: i + insertCount - removeCount).write(s.ptr.offset(by: i).read());
                i = i - Int64(intLiteral: 1)
            }
        } else if insertCount < removeCount {
            // Shift elements left
            var i = end;
            while i < myLen {
                s.ptr.offset(by: start + insertCount + (i - end)).write(s.ptr.offset(by: i).read());
                i = i + Int64(intLiteral: 1)
            }
        }

        // Copy replacement
        for i in 0..<insertCount {
            s.ptr.offset(by: start + i).write(replacement(unchecked: i))
        }

        s.len = newLen;
        self.storage.setValue(s)
    }

    /// Randomly shuffles the array in place using the provided generator.
    ///
    /// Uses the Fisher-Yates shuffle algorithm.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     var rng = Lcg64(seed: 42)
    ///     arr.shuffle(using: rng)  // deterministic shuffle
    public mutating func shuffle[R](using rng: R) where R: RandomNumberGenerator {
        let n = self.len();
        if n <= Int64(intLiteral: 1) {
            return
        }
        self.makeUnique();
        var s = self.storage.getValue();
        var generator = rng;

        // Fisher-Yates shuffle
        var i: Int64 = n - Int64(intLiteral: 1);
        while i > Int64(intLiteral: 0) {
            // Inline nextInt(below:) since extension methods may not be visible on generic R
            let bound = UInt64(from: i) + UInt64(intLiteral: 1);
            let rngValue = generator.nextUInt64();
            let j = Int64(from: rngValue.modulo(bound));
            // Swap elements at i and j
            let temp = s.ptr.offset(by: i).read();
            s.ptr.offset(by: i).write(s.ptr.offset(by: j).read());
            s.ptr.offset(by: j).write(temp);
            i = i - Int64(intLiteral: 1)
        }

        self.storage.setValue(s)
    }

    /// Randomly shuffles the array in place.
    ///
    /// Uses the default random number generator.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     arr.shuffle()  // e.g., [3, 1, 5, 2, 4]
    public mutating func shuffle() {
        var rng = Lcg64();
        self.shuffle(using: rng)
    }

    /// Returns a new array with elements in random order using the provided generator.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     var rng = Lcg64(seed: 42)
    ///     let result = arr.shuffled(using: rng)
    public func shuffled[R](using rng: R) -> Array[T] where R: RandomNumberGenerator {
        var result = self.clone();
        result.shuffle(using: rng);
        result
    }

    /// Returns a new array with elements in random order.
    ///
    /// Uses the default random number generator.
    /// The original array is unchanged.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     let shuffled = arr.shuffled()  // e.g., [4, 2, 5, 1, 3]
    ///     // arr is still [1, 2, 3, 4, 5]
    public func shuffled() -> Array[T] {
        var result = self.clone();
        result.shuffle();
        result
    }

    // ========================================================================
    // CAPACITY MANAGEMENT
    // ========================================================================

    /// Reserves capacity for at least minimumCapacity elements.
    ///
    /// Does nothing if current capacity is already sufficient.
    /// Use when you know many elements will be added.
    ///
    /// Example:
    ///     var arr = Array[Int64]()
    ///     arr.reserveCapacity(minimumCapacity: 1000)
    ///     for i in 0..<1000 {
    ///         arr.append( i)  // No reallocations
    ///     }
    public mutating func reserveCapacity(minimumCapacity: Int64) {
        self.grow(minimumCapacity)
    }

    /// Reduces capacity to match the current count.
    ///
    /// Frees excess memory. Useful after removing many elements.
    ///
    /// Example:
    ///     var arr = Array[Int64](capacity: 1000)
    ///     arr.append( 1)
    ///     arr.shrinkToFit()  // capacity reduced to ~1
    public mutating func shrinkToFit() {
        let myLen = self.len();
        let myCap = self.cap();
        if myLen == myCap or myLen == Int64(intLiteral: 0) {
            if myLen == Int64(intLiteral: 0) and myCap > Int64(intLiteral: 0) {
                // Deallocate entirely for empty array
                self.makeUnique();
                var s = self.storage.getValue();
                let layout = Layout.array[T](myCap);
                var allocator = SystemAllocator();
                allocator.deallocate(s.ptr.asRaw(), layout);
                s.ptr = Pointer(raw: lang.ptr_null[T]());
                s.cap = Int64(intLiteral: 0);
                self.storage.setValue(s)
            }
            return
        }

        self.makeUnique();

        // Reallocate to exact size
        let newLayout = Layout.array[T](myLen);
        var allocator = SystemAllocator();
        let result = allocator.allocate(newLayout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            let oldStorage = self.storage.getValue();
            for i in 0..<myLen {
                newPtr.offset(by: i).write(oldStorage.ptr.offset(by: i).read())
            }
            if myCap > Int64(intLiteral: 0) {
                let oldLayout = Layout.array[T](myCap);
                allocator.deallocate(oldStorage.ptr.asRaw(), oldLayout)
            }
            self.storage.setValue(ArrayStorage(ptr: newPtr, len: myLen, cap: myLen))
        }
    }

    // ========================================================================
    // ACCESSORS (continued)
    // ========================================================================

    /// Returns the first element, or None if empty.
    ///
    /// Example:
    ///     [1, 2, 3].first()  // Some(1)
    ///     [].first()         // None
    public func first() -> T? {
        if self.len() > Int64(intLiteral: 0) {
            .Some(self.ptr().read())
        } else {
            .None
        }
    }

    /// Returns the last element, or None if empty.
    ///
    /// Example:
    ///     [1, 2, 3].last()  // Some(3)
    ///     [].last()         // None
    public func last() -> T? {
        let myLen = self.len();
        if myLen > Int64(intLiteral: 0) {
            .Some(self.ptr().offset(by: myLen - Int64(intLiteral: 1)).read())
        } else {
            .None
        }
    }

    // ========================================================================
    // ITERATION
    // ========================================================================

    /// Returns an iterator over the array's elements.
    ///
    /// Example:
    ///     for item in arr.iter() { ... }
    ///     let doubled = arr.iter().map { it * 2 }.collect()
    public func iter() -> ArrayIterator[T] {
        ArrayIterator(ptr: self.ptr(), remaining: self.len())
    }

    // ========================================================================
    // SEARCHING
    // ========================================================================

    /// Returns the index of the first element matching the predicate, or None.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     arr.firstIndex(matching: { (x) in x > 3 })   // Some(3) - index of 4
    ///     arr.firstIndex(matching: { (x) in x > 10 })  // None
    public func firstIndex(matching predicate: (T) -> Bool) -> Int64? {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                return .Some(i)
            }
        }
        .None
    }

    /// Returns the index of the last element matching the predicate, or None.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 2, 1]
    ///     arr.lastIndex(matching: { (x) in x == 2 })  // Some(3)
    public func lastIndex(matching predicate: (T) -> Bool) -> Int64? {
        let myLen = self.len();
        if myLen == Int64(intLiteral: 0) {
            return .None
        }
        let myPtr = self.ptr();
        var i = myLen - Int64(intLiteral: 1);
        while i >= Int64(intLiteral: 0) {
            if predicate(myPtr.offset(by: i).read()) {
                return .Some(i)
            }
            i = i - Int64(intLiteral: 1)
        }
        .None
    }

    /// Returns the first element matching the predicate, or None.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     arr.first(matching: { (x) in x > 3 })  // Some(4)
    public func first(matching predicate: (T) -> Bool) -> T? {
        if let .Some(idx) = self.firstIndex(matching: predicate) {
            .Some(self(unchecked: idx))
        } else {
            .None
        }
    }

    /// Returns the last element matching the predicate, or None.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 2, 1]
    ///     arr.last(matching: { (x) in x > 1 })  // Some(2) - the second 2
    public func last(matching predicate: (T) -> Bool) -> T? {
        if let .Some(idx) = self.lastIndex(matching: predicate) {
            .Some(self(unchecked: idx))
        } else {
            .None
        }
    }

    // ========================================================================
    // PREDICATES
    // ========================================================================

    /// Returns true if all elements satisfy the predicate.
    ///
    /// Returns true for an empty array (vacuous truth).
    /// Short-circuits on first non-matching element.
    ///
    /// Example:
    ///     [2, 4, 6].all(satisfy: { (x) in x % 2 == 0 })  // true
    ///     [2, 3, 6].all(satisfy: { (x) in x % 2 == 0 })  // false
    ///     [].all(satisfy: { (x) in false })              // true (empty)
    public func all(satisfy predicate: (T) -> Bool) -> Bool {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) == false {
                return false
            }
        }
        true
    }

    /// Returns true if any element satisfies the predicate.
    ///
    /// Returns false for an empty array.
    /// Short-circuits on first matching element.
    ///
    /// Example:
    ///     [1, 2, 3].any(satisfy: { (x) in x > 2 })  // true
    ///     [1, 2, 3].any(satisfy: { (x) in x > 5 })  // false
    ///     [].any(satisfy: { (x) in true })          // false (empty)
    public func any(satisfy predicate: (T) -> Bool) -> Bool {
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                return true
            }
        }
        false
    }

    /// Returns the number of elements satisfying the predicate.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].countWhere({ (x) in x % 2 == 0 })  // 2
    public func countWhere(predicate: (T) -> Bool) -> Int64 {
        let myLen = self.len();
        let myPtr = self.ptr();
        var result: Int64 = Int64(intLiteral: 0);
        for i in 0..<myLen {
            if predicate(myPtr.offset(by: i).read()) {
                result = result + Int64(intLiteral: 1)
            }
        }
        result
    }

    // ========================================================================
    // SLICING
    // ========================================================================

    /// Returns a slice of the first count elements.
    ///
    /// Panics if count > self.count.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].prefix(count: 3)  // Slice[1, 2, 3]
    ///     [1, 2].prefix(count: 0)           // Empty slice
    public func prefix(count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            lang.panic("Array.prefix: count exceeds array length")
        }
        Slice(pointer: self.ptr(), count: count)
    }

    /// Returns a slice of the last count elements.
    ///
    /// Panics if count > self.count.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].suffix(count: 2)  // Slice[4, 5]
    public func suffix(count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            lang.panic("Array.suffix: count exceeds array length")
        }
        Slice(pointer: self.ptr().offset(by: myLen - count), count: count)
    }

    /// Returns a slice with the first count elements removed.
    ///
    /// Panics if count > self.count.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].drop(first: 2)  // Slice[3, 4, 5]
    public func drop(first count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            lang.panic("Array.drop(first:): count exceeds array length")
        }
        Slice(pointer: self.ptr().offset(by: count), count: myLen - count)
    }

    /// Returns a slice with the last count elements removed.
    ///
    /// Panics if count > self.count.
    ///
    /// Example:
    ///     [1, 2, 3, 4, 5].drop(last: 2)  // Slice[1, 2, 3]
    public func drop(last count: Int64) -> Slice[T] {
        let myLen = self.len();
        if count > myLen {
            lang.panic("Array.drop(last:): count exceeds array length")
        }
        Slice(pointer: self.ptr(), count: myLen - count)
    }

    // ========================================================================
    // CHUNKING
    // ========================================================================

    /// Returns an iterator over non-overlapping chunks of the given size.
    ///
    /// The last chunk may be smaller if count isn't evenly divisible.
    /// Panics if size is 0.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     for chunk in arr.chunks(of: 2) {
    ///         // yields [1, 2], then [3, 4], then [5]
    ///     }
    public func chunks(of size: Int64) -> ChunksIterator[T] {
        if size <= Int64(intLiteral: 0) {
            lang.panic("Array.chunks: size must be positive")
        }
        ChunksIterator(ptr: self.ptr(), remaining: self.len(), chunkSize: size)
    }

    /// Returns an iterator over overlapping sliding windows of the given size.
    ///
    /// Panics if size is 0 or size > count.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4]
    ///     for window in arr.windows(of: 2) {
    ///         // yields [1, 2], then [2, 3], then [3, 4]
    ///     }
    public func windows(of size: Int64) -> WindowsIterator[T] {
        if size <= Int64(intLiteral: 0) {
            lang.panic("Array.windows: size must be positive")
        }
        if size > self.len() {
            lang.panic("Array.windows: size exceeds array length")
        }
        WindowsIterator(ptr: self.ptr(), totalCount: self.len(), windowSize: size)
    }

    // ========================================================================
    // PARTITIONING
    // ========================================================================

    /// Reorders elements so those satisfying predicate come first.
    ///
    /// Returns the index of the first element not satisfying predicate
    /// (i.e., the partition point).
    /// The relative order within each partition is NOT preserved.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 4, 5]
    ///     let pivot = arr.partition(by: { (x) in x % 2 == 0 })
    ///     // arr might be [2, 4, 3, 1, 5] or similar
    ///     // pivot == 2 (first two elements satisfy predicate)
    public mutating func partition(by predicate: (T) -> Bool) -> Int64 {
        self.makeUnique();
        var s = self.storage.getValue();
        var lo: Int64 = Int64(intLiteral: 0);
        var hi: Int64 = s.len - Int64(intLiteral: 1);

        while true {
            // Find first element that doesn't satisfy predicate
            while lo < s.len and predicate(s.ptr.offset(by: lo).read()) {
                lo = lo + Int64(intLiteral: 1)
            }
            // Find last element that satisfies predicate
            while hi >= Int64(intLiteral: 0) and predicate(s.ptr.offset(by: hi).read()) == false {
                hi = hi - Int64(intLiteral: 1)
            }

            if lo >= hi {
                break
            }

            // Swap
            let temp = s.ptr.offset(by: lo).read();
            s.ptr.offset(by: lo).write(s.ptr.offset(by: hi).read());
            s.ptr.offset(by: hi).write(temp);
            lo = lo + Int64(intLiteral: 1);
            hi = hi - Int64(intLiteral: 1)
        }

        self.storage.setValue(s);
        lo
    }

    /// Splits into two arrays: elements satisfying predicate and those that don't.
    ///
    /// Preserves relative order within each resulting array.
    ///
    /// Example:
    ///     let (evens, odds) = [1, 2, 3, 4, 5].partitioned(by: { (x) in x % 2 == 0 })
    ///     // evens = [2, 4]
    ///     // odds = [1, 3, 5]
    public func partitioned(by predicate: (T) -> Bool) -> (Array[T], Array[T]) {
        var matching = Array[T]();
        var notMatching = Array[T]();
        let myLen = self.len();
        let myPtr = self.ptr();
        for i in 0..<myLen {
            let element = myPtr.offset(by: i).read();
            if predicate(element) {
                matching.append( element)
            } else {
                notMatching.append( element)
            }
        }
        (matching, notMatching)
    }

    // ========================================================================
    // PROTOCOL CONFORMANCES
    // ========================================================================

    /// Creates a shallow clone (COW - copy deferred until mutation).
    public func clone() -> Array[T] {
        Array(storage: self.storage.clone())
    }
}

// ============================================================================
// CONDITIONAL EXTENSIONS
// ============================================================================

/// Equatable extension when T is Equatable.
extend Array[T]: Equatable where T: Equatable {
    /// Compares two arrays for equality.
    ///
    /// Two arrays are equal if they have the same length and all
    /// corresponding elements are equal.
    ///
    /// Example:
    ///     [1, 2, 3].equals(other: [1, 2, 3])  // true
    ///     [1, 2, 3].equals(other: [1, 2])     // false
    ///     [1, 2, 3].equals(other: [3, 2, 1])  // false
    public func equals(other: Array[T]) -> Bool {
        let selfCount = self.count;
        let otherCount = other.count;
        if selfCount != otherCount {
            return false
        }
        var i: Int64 = Int64(intLiteral: 0);
        var equal: Bool = true;
        while i < selfCount and equal {
            if self(unchecked: i).equals(other(unchecked: i)) == false {
                equal = false
            }
            i = i + Int64(intLiteral: 1)
        }
        equal
    }

    /// Returns true if the array contains the given element.
    ///
    /// Example:
    ///     [1, 2, 3].contains(element: 2)  // true
    ///     [1, 2, 3].contains(element: 5)  // false
    public func contains(element: T) -> Bool {
        self.firstIndex(matching: { (x) in x.equals(element) }).isSome()
    }

    /// Returns the index of the first occurrence of element, or None.
    ///
    /// Example:
    ///     [1, 2, 3, 2].firstIndex(of: 2)  // Some(1)
    ///     [1, 2, 3].firstIndex(of: 5)     // None
    public func firstIndex(of element: T) -> Int64? {
        self.firstIndex(matching: { (x) in x.equals(element) })
    }

    /// Returns the index of the last occurrence of element, or None.
    ///
    /// Example:
    ///     [1, 2, 3, 2].lastIndex(of: 2)  // Some(3)
    public func lastIndex(of element: T) -> Int64? {
        self.lastIndex(matching: { (x) in x.equals(element) })
    }

    /// Returns true if the array starts with the given prefix.
    ///
    /// Example:
    ///     [1, 2, 3].starts(with: [1, 2])     // true
    ///     [1, 2, 3].starts(with: [1, 2, 3])  // true
    ///     [1, 2, 3].starts(with: [2, 3])     // false
    ///     [1, 2].starts(with: [1, 2, 3])     // false (prefix longer)
    ///     [1, 2, 3].starts(with: [])         // true (empty prefix)
    public func starts(with prefix: Array[T]) -> Bool {
        let prefixLen = prefix.count;
        if prefixLen > self.count {
            return false
        }
        for i in 0..<prefixLen {
            if self(unchecked: i).equals(prefix(unchecked: i)) == false {
                return false
            }
        }
        true
    }

    /// Returns true if the array ends with the given suffix.
    ///
    /// Example:
    ///     [1, 2, 3].ends(with: [2, 3])  // true
    ///     [1, 2, 3].ends(with: [1, 2])  // false
    ///     [1, 2, 3].ends(with: [])      // true (empty suffix)
    public func ends(with suffix: Array[T]) -> Bool {
        let suffixLen = suffix.count;
        let myLen = self.count;
        if suffixLen > myLen {
            return false
        }
        let offset = myLen - suffixLen;
        for i in 0..<suffixLen {
            if self(unchecked: offset + i).equals(suffix(unchecked: i)) == false {
                return false
            }
        }
        true
    }

    /// Splits the array at each occurrence of separator.
    ///
    /// The separator elements are not included in the resulting slices.
    /// Empty slices are included when separators are adjacent or at edges.
    ///
    /// Example:
    ///     [1, 0, 2, 0, 3].split(separator: 0)
    ///     // [Slice[1], Slice[2], Slice[3]]
    ///
    ///     [0, 1, 0, 0, 2, 0].split(separator: 0)
    ///     // [Slice[], Slice[1], Slice[], Slice[2], Slice[]]
    ///
    ///     [1, 2, 3].split(separator: 0)
    ///     // [Slice[1, 2, 3]] - no separator found
    ///
    ///     [].split(separator: 0)
    ///     // [Slice[]] - empty array yields one empty slice
    public func split(separator: T) -> Array[Slice[T]] {
        var result = Array[Slice[T]]();
        let myLen = self.count;
        var start: Int64 = Int64(intLiteral: 0);
        for i in 0..<myLen {
            if self(unchecked: i).equals(separator) {
                result.append( Slice(pointer: self.asPointer().offset(by: start), count: i - start));
                start = i + Int64(intLiteral: 1)
            }
        }
        result.append( Slice(pointer: self.asPointer().offset(by: start), count: myLen - start));
        result
    }

    /// Removes the first occurrence of element.
    ///
    /// Returns true if the element was found and removed, false otherwise.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 2]
    ///     arr.remove(element: 2)  // true, arr is [1, 3, 2]
    ///     arr.remove(element: 5)  // false, arr unchanged
    public mutating func remove(element: T) -> Bool {
        if let .Some(idx) = self.firstIndex(matching: { (x) in x.equals(element) }) {
            let _ = self.remove(at: idx);
            true
        } else {
            false
        }
    }

    /// Removes all occurrences of element.
    ///
    /// Example:
    ///     var arr = [1, 2, 3, 2, 4, 2]
    ///     arr.removeAll(element: 2)  // arr is [1, 3, 4]
    public mutating func removeAll(element: T) {
        self.retain(matching: { (x) in x.equals(element) == false })
    }

    /// Removes consecutive duplicate elements in place.
    ///
    /// Only removes duplicates that are adjacent. Sort first for full dedup.
    ///
    /// Example:
    ///     var arr = [1, 1, 2, 2, 2, 3, 1, 1]
    ///     arr.dedup()  // [1, 2, 3, 1] - note trailing 1s kept
    public mutating func dedup() {
        if self.count <= Int64(intLiteral: 1) {
            return
        }
        self.makeUnique();
        var s = self.storage.getValue();
        var writeIdx: Int64 = Int64(intLiteral: 1);
        for readIdx in 1..<s.len {
            let current = s.ptr.offset(by: readIdx).read();
            let previous = s.ptr.offset(by: writeIdx - Int64(intLiteral: 1)).read();
            if current.equals(previous) == false {
                if writeIdx != readIdx {
                    s.ptr.offset(by: writeIdx).write(current)
                }
                writeIdx = writeIdx + Int64(intLiteral: 1)
            }
        }
        s.len = writeIdx;
        self.storage.setValue(s)
    }

    /// Returns a new array with consecutive duplicates removed.
    ///
    /// Example:
    ///     [1, 1, 2, 2, 3].deduped()  // [1, 2, 3]
    public func deduped() -> Array[T] {
        var result = self.clone();
        result.dedup();
        result
    }
}

/// ArrayMatchable extension for array pattern matching.
/// Enables patterns like `[a, b]`, `[a, ..rest]`, `[a, .., z]`, `[a, ..rest, z]`.
extend Array[T]: ArrayMatchable {
    type Element = T

    /// Returns the number of elements in the array.
    public func matchLength() -> Int64 {
        self.count
    }

    /// Returns the element at the given index (unchecked).
    public func matchGet(index: Int64) -> T {
        self(unchecked: index)
    }

    /// Returns a slice from `from` (inclusive) to `to` (exclusive).
    public func matchSlice(from: Int64, to: Int64) -> Slice[T] {
        Slice(pointer: self.asPointer().offset(by: from), count: to - from)
    }
}

// ============================================================================
// COMPARABLE EXTENSION
// ============================================================================

/// Comparable extension for arrays with comparable elements.
extend Array[T] where T: Comparable {
    /// Sorts the array in place in ascending order.
    ///
    /// Example:
    ///     var arr = [3, 1, 4, 1, 5]
    ///     arr.sort()  // [1, 1, 3, 4, 5]
    public mutating func sort() {
        self.sort(by: { (a, b) in a < b })
    }

    /// Returns a new sorted array in ascending order.
    ///
    /// The original array is unchanged.
    ///
    /// Example:
    ///     let arr = [3, 1, 4, 1, 5]
    ///     let sorted = arr.sorted()  // [1, 1, 3, 4, 5]
    ///     // arr is still [3, 1, 4, 1, 5]
    public func sorted() -> Array[T] {
        self.sorted(by: { (a, b) in a < b })
    }

    /// Returns the minimum element, or None if empty.
    ///
    /// Example:
    ///     [3, 1, 4].min()  // Some(1)
    ///     [].min()         // None
    public func min() -> T? {
        if self.count == Int64(intLiteral: 0) {
            return .None
        }
        var result = self(unchecked: Int64(intLiteral: 0));
        for i in 1..<self.count {
            let element = self(unchecked: i);
            if element < result {
                result = element
            }
        }
        .Some(result)
    }

    /// Returns the maximum element, or None if empty.
    ///
    /// Example:
    ///     [3, 1, 4].max()  // Some(4)
    ///     [].max()         // None
    public func max() -> T? {
        if self.count == Int64(intLiteral: 0) {
            return .None
        }
        var result = self(unchecked: Int64(intLiteral: 0));
        for i in 1..<self.count {
            let element = self(unchecked: i);
            if element > result {
                result = element
            }
        }
        .Some(result)
    }

    /// Returns true if the array is sorted in ascending order.
    ///
    /// An empty array and single-element array are considered sorted.
    ///
    /// Example:
    ///     [1, 2, 3].isSorted()  // true
    ///     [1, 3, 2].isSorted()  // false
    ///     [1, 1, 1].isSorted()  // true (equal elements OK)
    ///     [].isSorted()         // true
    public func isSorted() -> Bool {
        if self.count <= Int64(intLiteral: 1) {
            return true
        }
        for i in 1..<self.count {
            if self(unchecked: i) < self(unchecked: i - Int64(intLiteral: 1)) {
                return false
            }
        }
        true
    }

    /// Performs binary search for element.
    ///
    /// Returns the index if found, or None if not found.
    /// WARNING: The array MUST be sorted; behavior is undefined otherwise.
    ///
    /// Example:
    ///     let arr = [1, 2, 3, 4, 5]
    ///     arr.binarySearch(element: 3)  // Some(2)
    ///     arr.binarySearch(element: 6)  // None
    public func binarySearch(element: T) -> Int64? {
        var lo: Int64 = Int64(intLiteral: 0);
        var hi: Int64 = self.count;
        while lo < hi {
            let mid = lo + (hi - lo) / Int64(intLiteral: 2);
            let midVal = self(unchecked: mid);
            if midVal < element {
                lo = mid + Int64(intLiteral: 1)
            } else if midVal > element {
                hi = mid
            } else {
                return .Some(mid)
            }
        }
        .None
    }
}

// ============================================================================
// HASH EXTENSION
// ============================================================================

/// Hash extension for arrays with hashable elements.
extend Array[T] where T: Hash {
    /// Returns a new array with all duplicates removed.
    ///
    /// Preserves the order of first occurrences.
    /// Note: Uses O(n^2) algorithm. For O(n) performance, use a Set.
    ///
    /// Example:
    ///     [1, 2, 1, 3, 2, 4].unique()  // [1, 2, 3, 4]
    public func unique() -> Array[T] {
        var result = Array[T]();
        let myLen = self.count;
        for i in 0..<myLen {
            let element = self(unchecked: i);
            var found = false;
            for j in 0..<result.count {
                if result(unchecked: j).equals(element) {
                    found = true
                }
            }
            if found == false {
                result.append( element)
            }
        }
        result
    }

    /// Removes all duplicate elements in place.
    ///
    /// Preserves the order of first occurrences.
    ///
    /// Example:
    ///     var arr = [1, 2, 1, 3, 2]
    ///     arr.removeDuplicates()  // [1, 2, 3]
    public mutating func removeDuplicates() {
        self = self.unique()
    }
}

// ============================================================================
// CUSTOM SORTING EXTENSION
// ============================================================================

/// Custom sorting methods for all arrays.
extend Array[T] {
    /// Sorts the array in place using a custom comparator.
    ///
    /// The comparator should return true if the first argument should
    /// come before the second.
    ///
    /// Example:
    ///     var arr = [1, 5, 3, 2, 4]
    ///     arr.sort(by: { (a, b) in a > b })  // [5, 4, 3, 2, 1] descending
    public mutating func sort(by comparator: (T, T) -> Bool) {
        let n = self.count;
        if n <= Int64(intLiteral: 1) {
            return
        }
        self.makeUnique();
        // Insertion sort (simple and stable)
        for i in 1..<n {
            let key = self(unchecked: i);
            var j = i - Int64(intLiteral: 1);
            while j >= Int64(intLiteral: 0) and comparator(key, self(unchecked: j)) {
                self.setUnchecked(j + Int64(intLiteral: 1), self(unchecked: j));
                j = j - Int64(intLiteral: 1)
            }
            self.setUnchecked(j + Int64(intLiteral: 1), key)
        }
    }

    /// Returns a new array sorted using a custom comparator.
    ///
    /// Example:
    ///     let arr = ["apple", "Banana", "cherry"]
    ///     let sorted = arr.sorted(by: { (a, b) in a.lowercase() < b.lowercase() })
    public func sorted(by comparator: (T, T) -> Bool) -> Array[T] {
        var result = self.clone();
        result.sort(by: comparator);
        result
    }

    /// Sorts the array in place by a key extracted from each element.
    ///
    /// Example:
    ///     var people = [Person("Alice", 30), Person("Bob", 25)]
    ///     people.sort(byKey: { (p) in p.age })  // sorted by age ascending
    public mutating func sort[K](byKey key: (T) -> K) where K: Comparable {
        self.sort(by: { (a, b) in key(a) < key(b) })
    }

    /// Returns a new array sorted by a key extracted from each element.
    ///
    /// Example:
    ///     let words = ["hi", "hello", "hey"]
    ///     let byLength = words.sorted(byKey: { (w) in w.count })  // ["hi", "hey", "hello"]
    public func sorted[K](byKey key: (T) -> K) -> Array[T] where K: Comparable {
        self.sorted(by: { (a, b) in key(a) < key(b) })
    }
}

// ============================================================================
// NESTED STRUCTURE EXTENSIONS
// ============================================================================

/// Extension for arrays of iterable elements.
extend Array[T] where T: Iterable {
    /// Flattens nested iterables into a single array.
    ///
    /// Example:
    ///     let nested = [[1, 2], [3, 4], [5]]
    ///     nested.flatten()  // [1, 2, 3, 4, 5]
    ///
    ///     let mixed = [[1], [], [2, 3]]
    ///     mixed.flatten()  // [1, 2, 3]
    public func flatten() -> Array[T.Item] {
        var result = Array[T.Item]();
        for i in 0..<self.count {
            var iter = self(unchecked: i).iter();
            while let .Some(item) = iter.next() {
                result.append( item)
            }
        }
        result
    }
}

/// Extension for arrays of formattable elements.
extend Array[T] where T: Formattable {
    /// Joins elements into a string with the given separator.
    ///
    /// Each element is converted to string via Formattable.
    ///
    /// Example:
    ///     [1, 2, 3].joined(separator: ", ")  // "1, 2, 3"
    ///     [1, 2, 3].joined()                 // "123"
    ///     ["a", "b"].joined(separator: "-")  // "a-b"
    ///     [].joined(separator: ", ")         // ""
    public func joined(separator: String = "") -> String {
        if self.count == Int64(intLiteral: 0) {
            return ""
        }
        var result = self(unchecked: Int64(intLiteral: 0)).format();
        for i in 1..<self.count {
            result = result + separator;
            result = result + self(unchecked: i).format()
        }
        result
    }
}

// ============================================================================
// FORMATTABLE CONFORMANCE
// ============================================================================

/// Formattable conformance for arrays with formattable elements.
///
/// Arrays format as "[elem1, elem2, ...]".
///
/// Example:
///     "\{[1, 2, 3]}"  // "[1, 2, 3]"
extend Array[T]: Formattable where T: Formattable {
    public func format(options: FormatOptions = FormatOptions.default()) -> String {
        var result = "[";
        let myLen = self.count;
        for i in 0..<myLen {
            if i > Int64(intLiteral: 0) {
                result = result + ", "
            }
            result = result + self(unchecked: i).format(options)
        }
        result = result + "]";
        result
    }
}

// ============================================================================
// DIRECT ITERABLE CONFORMANCE
// ============================================================================

// TODO: DirectIterable protocol not yet implemented
// /// DirectIterable conformance allows using iterator methods directly on arrays.
// extend Array[T]: DirectIterable[T] {
//     public static func collect[I](from iter: I) -> Array[T] where I: Iterator, I.Item = T {
//         var result = Array[T]();
//         var iterator = iter;
//         while let .Some(item) = iterator.next() {
//             result.append( item)
//         }
//         result
//     }
// }

// ============================================================================
// TYPE OPERATOR
// ============================================================================

/// Type operator alias: [T] desugars to Array[T].
@builtin(.ArrayTypeOperator)
public type ArrayTypeOperator[T] = Array[T];
