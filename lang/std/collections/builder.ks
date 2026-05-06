// ArrayBuilder[T] - write-only buffer for efficient array construction

module std.collections

import std.core.(Bool, fatalError)
import std.numeric.(Int64)
import std.memory.(Layout, Pointer, ArraySlice, SystemAllocator, RcBox)
import std.iter.(Iterable)

/// Write-only buffer for efficient array construction. No COW, no
/// `RcBox`, no `isUnique` checks — every append writes directly through
/// the pointer.
///
/// `build()` transfers ownership of the buffer into a new `Array[T]`
/// without copying. The builder resets to empty and can be reused.
///
/// # Examples
///
/// ```
/// var b = ArrayBuilder[Int64](capacity: 3);
/// b.append(1);
/// b.append(2);
/// b.append(3);
/// let arr = b.build();   // [1, 2, 3], zero-copy
/// ```
///
/// # Representation
///
/// `(ptr: Pointer[T], len: Int64, cap: Int64)`.
///
/// # Memory Model
///
/// Owns its buffer directly — no reference counting during
/// construction. `build()` donates the buffer to an `Array[T]` and
/// leaves the builder empty. `deinit` frees the buffer if `build()`
/// was never called.
public struct ArrayBuilder[T] {
    fileprivate var ptr: Pointer[T]
    fileprivate var len: Int64
    fileprivate var cap: Int64

    /// @name Empty
    /// Creates an empty builder with no allocation.
    public init() {
        self.ptr = Pointer[T].nullPointer();
        self.len = 0;
        self.cap = 0;
    }

    /// @name With Capacity
    /// Creates an empty builder with at least `capacity` elements
    /// preallocated.
    public init(capacity capacity: Int64) {
        if capacity > 0 {
            let layout = Layout.array[T](capacity);
            var allocator = SystemAllocator();
            let result = allocator.allocate(layout);
            if let .Some(rawPtr) = result {
                self.ptr = rawPtr.cast[T]();
                self.len = 0;
                self.cap = capacity
            } else {
                fatalError("ArrayBuilder allocation failed")
            }
        } else {
            self.ptr = Pointer[T].nullPointer();
            self.len = 0;
            self.cap = 0
        }
    }

    // -- Growth --------------------------------------------------------------

    private mutating func grow(minCapacity: Int64) {
        if self.cap >= minCapacity { return }
        var newCap = self.cap;
        if newCap == 0 { newCap = 4 }
        while newCap < minCapacity {
            newCap = newCap * 2
        }
        let layout = Layout.array[T](newCap);
        var allocator = SystemAllocator();
        let result = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            let newPtr = rawPtr.cast[T]();
            for i in 0..<self.len {
                newPtr.offset(by: i).write(self.ptr.offset(by: i).read())
            }
            if self.cap > 0 {
                let oldLayout = Layout.array[T](self.cap);
                allocator.deallocate(self.ptr.asRaw(), oldLayout)
            }
            self.ptr = newPtr;
            self.cap = newCap
        } else {
            fatalError("ArrayBuilder grow failed")
        }
    }

    // -- Appending -----------------------------------------------------------

    /// Appends a single element.
    public mutating func append(element: T) {
        self.grow(self.len + 1);
        self.ptr.offset(by: self.len).write(element);
        self.len = self.len + 1
    }

    /// Appends every element of `slice`.
    public mutating func append(contentsOf slice: ArraySlice[T]) {
        let n = slice.count;
        if n == 0 { return }
        self.grow(self.len + n);
        let src = slice.pointer;
        for i in 0..<n {
            self.ptr.offset(by: self.len + i).write(src.offset(by: i).read())
        }
        self.len = self.len + n
    }

    /// Appends every element produced by `iterable`.
    public mutating func appendFrom[I](iterable: I) where I: Iterable, I.Item = T {
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    // -- Build ---------------------------------------------------------------

    /// Transfers the buffer into a new `Array[T]` without copying. The
    /// builder resets to empty and can be reused.
    public mutating func build() -> Array[T] {
        if self.len == 0 {
            return Array[T]()
        }
        let storage = ArrayStorage(ptr: self.ptr, len: self.len, cap: self.cap);
        let result = Array[T](storage: RcBox(storage));
        self.ptr = Pointer[T].nullPointer();
        self.len = 0;
        self.cap = 0;
        result
    }

    /// Resets length to zero, keeping the allocated buffer for reuse.
    public mutating func clear() {
        self.len = 0
    }

    // -- Queries -------------------------------------------------------------

    /// Number of elements written so far.
    public var count: Int64 { self.len }

    /// True when nothing has been written.
    public var isEmpty: Bool { self.len == 0 }

    /// Allocated capacity in elements.
    public var capacity: Int64 { self.cap }

    // -- Cleanup -------------------------------------------------------------

    deinit {
        if self.cap > 0 {
            let layout = Layout.array[T](self.cap);
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}
