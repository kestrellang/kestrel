// ArrayBuilder[T] - write-only buffer for efficient array construction

module std.collections

import std.core.(Bool, Defaultable, fatalError)
import std.numeric.(Int64)
import std.memory.(Layout, Pointer, Allocator, GlobalAllocator, CowBox)
import std.ffi.(memcpy)
import std.collections.(Sliceable)
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
public struct ArrayBuilder[T, A = GlobalAllocator] where A: Allocator {
    fileprivate var ptr: Pointer[T]
    fileprivate var len: Int64
    fileprivate var cap: Int64
    /// Where the buffer comes from; handed to the built `Array`.
    fileprivate var allocator: A

    /// @name With Allocator
    /// An empty builder whose buffer (and the built array's) comes from
    /// `allocator`. Nothing is allocated until the first append.
    public init(allocator allocator: A) {
        self.ptr = Pointer[T].nullPointer();
        self.len = 0;
        self.cap = 0;
        self.allocator = allocator;
    }

    /// @name With Capacity And Allocator
    /// An empty builder with room for `capacity` elements, allocated from
    /// `allocator`. Panics on allocation failure.
    public init(capacity capacity: Int64, allocator allocator: A) {
        self.init(allocator: allocator);
        self.grow(capacity)
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
        // A move is a bit copy, so `reallocate` moves the elements (often
        // in place, with no copy at all).
        let result = if self.cap == 0 {
            self.allocator.allocate(layout)
        } else {
            unsafe self.allocator.reallocate(self.ptr.asRaw(), Layout.array[T](self.cap), layout)
        };
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]();
            self.cap = newCap
        } else {
            fatalError("ArrayBuilder grow failed")
        }
    }

    // -- Appending -----------------------------------------------------------

    /// Appends a single element.
    public mutating func append(element: T) {
        self.grow(self.len + 1);
        unsafe self.ptr.offset(by: self.len).write(element);
        self.len = self.len + 1
    }

    /// Appends every element of `other` (an `Array`, `Slice`, or any
    /// other `Sliceable`), copying in a single pass.
    public mutating func append(contentsOf other: &some Sliceable[T] and not Static and not Copyable and not Sized) {
        let sl = other.asUnsafeSlice();
        let n = sl.count;
        if n == 0 { return }
        self.grow(self.len + n);
        let src = sl.pointer;
        if Bool(boolLiteral: lang.is_bitwise[T]()) {
            let _ = unsafe memcpy(self.ptr.offset(by: self.len).asRaw(), src.asRaw(), n * Layout.of[T]().size);
        } else {
            for i in 0..<n {
                unsafe self.ptr.offset(by: self.len + i).write(src.offset(by: i).read())
            }
        }
        self.len = self.len + n
    }

    /// Appends every element produced by `iterable`.
    public mutating func append[I](from iterable: I) where I: Iterable, I: not Static, I.Item = T {
        var iter = iterable.iter();
        while let .Some(item) = iter.next() {
            self.append(item)
        }
    }

    // -- Build ---------------------------------------------------------------

    /// Transfers the buffer into a new `Array[T]` without copying. The
    /// builder resets to empty and can be reused.
    public mutating func build() -> Array[T, A] {
        if self.len == 0 {
            return Array[T, A](allocator: self.allocator)
        }
        let storage = ArrayStorage(ptr: self.ptr, len: self.len, cap: self.cap, allocator: self.allocator);
        let result = Array[T, A](storage: CowBox(storage));
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

    /// Drops any unbuilt elements, then frees the buffer.
    ///
    /// Only runs when the builder is dropped *without* `build()` — e.g.
    /// construction abandoned on an early return. `build()` transfers the
    /// buffer into an `Array` and resets `cap` to 0, so this is a no-op on a
    /// built builder (the array's storage owns the elements then). On the
    /// abandoned path the `len` written elements are still live and must be
    /// dropped, or every non-trivial element (e.g. a `String`) leaks. The
    /// builder is not `Cloneable`, so there is no aliasing copy to double-free.
    deinit {
        if self.cap > 0 {
            var i: Int64 = 0;
            while i < self.len {
                unsafe self.ptr.offset(by: i).dropInPlace();
                i = i + 1
            };
            let layout = Layout.array[T](self.cap);
            var allocator = self.allocator;
            unsafe allocator.deallocate(self.ptr.asRaw(), layout)
        }
    }
}

// Constructors that make their own allocator.
extend ArrayBuilder[T, A] where A: Defaultable {
    /// @name Empty
    /// Creates an empty builder with no allocation.
    public init() {
        self.init(allocator: A())
    }

    /// @name With Capacity
    /// Creates an empty builder with at least `capacity` elements
    /// preallocated.
    public init(capacity capacity: Int64) {
        self.init(capacity: capacity, allocator: A())
    }
}
