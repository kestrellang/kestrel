// Buffer[T, A] - owning contiguous memory region

module std.memory

import std.result.(Optional)
import std.core.(Bool, Copyable, fatalError)
import std.numeric.(Int64)
import std.memory.(Layout, Pointer, ArraySlice, RawPointer, Allocator, GlobalAllocator)
import std.ffi.(memcpy, memmove, memset)

/// Owning, allocator-parameterised contiguous storage.
///
/// `Buffer` is the building block underneath `Array`, `String`, and any
/// other COW/growable collection. It owns its allocation, deallocates on
/// drop, and is `not Copyable` to keep ownership unique. For a non-owning
/// view see `Slice`; for a refcounted owning wrapper see `RcBox`.
///
/// # Examples
///
/// ```
/// var buf = Buffer[Int64, SystemAllocator](capacity: 4, allocator: SystemAllocator());
/// buf.write(at: 0, value: 10);
/// buf.write(at: 1, value: 20);
/// buf.read(at: 0)              // .Some(10)
/// buf.resize(to: 8);           // grow in place if possible
/// ```
///
/// # Representation
///
/// A `Pointer[T]` to the storage, an `Int64` capacity, and the allocator
/// instance. The buffer's contents are not initialised on construction —
/// reading an uninitialised slot is undefined behavior.
///
/// # Memory Model
///
/// Owning, unique. The deinit reclaims storage via the same allocator.
/// Marked `not Copyable` so an accidental `let b2 = b1` is rejected at
/// compile time; use a higher-level COW wrapper (e.g. via `RcBox`) for
/// shared semantics.
public struct Buffer[T, A]: not Copyable where A: Allocator {
    private var ptr: Pointer[T]
    private var cap: Int64
    private var allocator: A

    /// @name With Capacity
    /// Allocates a buffer holding `capacity` elements. Storage is
    /// uninitialised; the caller is responsible for writing valid `T`s
    /// before reading them.
    ///
    /// # Errors
    ///
    /// Panics with `"Buffer allocation failed"` if `allocator.allocate`
    /// returns `.None`.
    public init(capacity: Int64, allocator: A) {
        self.allocator = allocator;
        self.cap = capacity;
        let layout = Layout.array[T](capacity);
        let result = self.allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]()
        } else {
            fatalError("Buffer allocation failed")
        }
    }

    // Internal init that adopts an existing allocation. Used by
    // higher-level collections that already hold the storage and want
    // to wrap it as a Buffer (e.g. when reconstructing from raw fields).
    private init(pointer: Pointer[T], capacity: Int64, allocator: A) {
        self.ptr = pointer;
        self.cap = capacity;
        self.allocator = allocator;
    }

    /// Releases the storage through the same allocator instance the
    /// buffer was constructed with.
    deinit {
        let layout = Layout.array[T](self.cap);
        self.allocator.deallocate(self.ptr.asRaw(), layout)
    }

    /// Number of element slots — not the count of *initialised* elements.
    public var capacity: Int64 { self.cap }

    /// Pointer to the first slot.
    public var pointer: Pointer[T] { self.ptr }

    /// @name Unchecked Index
    /// Reads slot `index` without bounds checking.
    ///
    /// # Safety
    ///
    /// `index` must satisfy `0 <= index < capacity`, and the slot must
    /// already hold an initialised `T`. Out-of-range or uninitialised
    /// reads are undefined behavior.
    public func read(unchecked index: Int64) -> T {
        self.ptr.offset(by: index).read()
    }

    /// @name Unchecked Index
    /// Writes `value` into slot `index` without bounds checking.
    ///
    /// # Safety
    ///
    /// Same precondition as `read(unchecked:)` — `0 <= index < capacity`.
    public func write(unchecked index: Int64, value: T) {
        self.ptr.offset(by: index).write(value)
    }

    /// @name Checked Index
    /// Reads slot `index`, returning `.None` when out of range. As with
    /// the unchecked form, the slot must already hold an initialised `T`.
    public func read(at index: Int64) -> T? {
        if index >= 0 and index < self.cap {
            .Some(self.ptr.offset(by: index).read())
        } else {
            .None
        }
    }

    /// Writes `value` to slot `index`. Returns `false` (and does
    /// nothing) when out of range.
    public func write(at index: Int64, value: T) -> Bool {
        if index >= 0 and index < self.cap {
            self.ptr.offset(by: index).write(value);
            true
        } else {
            false
        }
    }

    // Bulk operations
    // Note: These are commented out due to issues accessing .raw on expressions
    // public func copy(from source: Pointer[T], count: Int64) {
    //     let copyCount = if count < self.cap { count } else { self.cap };
    //     let elementSize = Int64(intLiteral: lang.sizeof[T]());
    //     let byteCount: Int64 = copyCount * elementSize;
    //     memcpy(self.ptr.asRaw().raw, source.asRaw().raw, byteCount.raw);
    // }

    // public func move(from source: Pointer[T], count: Int64) {
    //     let moveCount = if count < self.cap { count } else { self.cap };
    //     let elementSize = Int64(intLiteral: lang.sizeof[T]());
    //     let byteCount: Int64 = moveCount * elementSize;
    //     memmove(self.ptr.asRaw().raw, source.asRaw().raw, byteCount.raw);
    // }

    // public func zeroFill() {
    //     let elementSize = Int64(intLiteral: lang.sizeof[T]());
    //     let byteCount: Int64 = self.cap * elementSize;
    //     memset(self.ptr.asRaw().raw, 0, byteCount.raw);
    // }

    /// Grows or shrinks the storage to hold `newCapacity` elements via
    /// the allocator's `reallocate`. On success, existing initialised
    /// elements are preserved up to the smaller of the two capacities;
    /// the new pointer becomes the buffer's storage.
    ///
    /// # Errors
    ///
    /// Panics with `"Buffer resize failed"` if `reallocate` returns
    /// `.None` (the original allocation is left intact, but the panic
    /// aborts).
    public mutating func resize(to newCapacity: Int64) {
        let oldLayout = Layout.array[T](self.cap);
        let newLayout = Layout.array[T](newCapacity);

        let result = self.allocator.reallocate(self.ptr.asRaw(), oldLayout, newLayout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]();
            self.cap = newCapacity
        } else {
            fatalError("Buffer resize failed")
        }
    }

    /// Returns a `ArraySlice[T]` over the entire buffer. The slice does not
    /// extend the buffer's lifetime; callers must keep the buffer alive
    /// for as long as they use the slice.
    public func asSlice() -> ArraySlice[T] {
        ArraySlice(pointer: self.ptr, count: self.cap)
    }

    /// Returns a slice over `[start, end)`, or `.None` when the range
    /// falls outside `[0, capacity]`. As with `asSlice`, the slice
    /// borrows from the buffer.
    public func slice(from start: Int64, to end: Int64) -> ArraySlice[T]? {
        if start >= 0 and end <= self.cap and start <= end {
            .Some(ArraySlice(pointer: self.ptr.offset(by: start), count: end - start))
        } else {
            .None
        }
    }
}
