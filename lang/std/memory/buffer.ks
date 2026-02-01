// Buffer[T, A] - owning contiguous memory region

module std.memory

import std.result.(Optional)
import std.core.(Bool, Copyable)
import std.num.(Int64)
import std.memory.(Layout, Pointer, Slice, RawPointer, Allocator, GlobalAllocator)
import std.ffi.(memcpy, memmove, memset)

/// A contiguous memory region that owns its allocation.
/// Not copyable to ensure unique ownership of the underlying memory.
/// Used as a building block for higher-level collections.
public struct Buffer[T, A]: not Copyable where A: Allocator {
    private var ptr: Pointer[T]
    private var cap: Int64
    private var allocator: A

    /// Allocates a buffer with the specified capacity using the provided allocator.
    /// Panics if allocation fails.
    public init(capacity: Int64, allocator: A) {
        self.allocator = allocator;
        self.cap = capacity;
        let layout = Layout.array[T](capacity);
        let result = self.allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]()
        } else {
            lang.panic("Buffer allocation failed")
        }
    }

    // Create from existing pointer (takes ownership)
    private init(pointer: Pointer[T], capacity: Int64, allocator: A) {
        self.ptr = pointer;
        self.cap = capacity;
        self.allocator = allocator;
    }

    /// Destructor: deallocates the buffer memory.
    deinit {
        let layout = Layout.array[T](self.cap);
        self.allocator.deallocate(self.ptr.asRaw(), layout)
    }

    /// The number of elements this buffer can hold.
    public var capacity: Int64 { self.cap }

    /// A pointer to the buffer's element storage.
    public var pointer: Pointer[T] { self.ptr }

    /// Reads the element at the given index without bounds checking.
    /// Undefined behavior if index is out of bounds.
    public func read(unchecked index: Int64) -> T {
        self.ptr.offset(by: index).read()
    }

    /// Writes a value at the given index without bounds checking.
    /// Undefined behavior if index is out of bounds.
    public func write(unchecked index: Int64, value: T) {
        self.ptr.offset(by: index).write(value)
    }

    /// Reads the element at the given index with bounds checking.
    /// Returns None if index is out of bounds.
    public func read(at index: Int64) -> T? {
        if index >= 0 and index < self.cap {
            .Some(self.ptr.offset(by: index).read())
        } else {
            .None
        }
    }

    /// Writes a value at the given index with bounds checking.
    /// Returns true on success, false if index is out of bounds.
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

    /// Resizes the buffer to the new capacity.
    /// Panics if reallocation fails.
    public mutating func resize(to newCapacity: Int64) {
        let oldLayout = Layout.array[T](self.cap);
        let newLayout = Layout.array[T](newCapacity);

        let result = self.allocator.reallocate(self.ptr.asRaw(), oldLayout, newLayout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[T]();
            self.cap = newCapacity
        } else {
            lang.panic("Buffer resize failed")
        }
    }

    /// Returns a slice view of the entire buffer.
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr, count: self.cap)
    }

    /// Returns a slice view of a portion of the buffer.
    /// Returns None if the range is out of bounds.
    public func slice(from start: Int64, to end: Int64) -> Slice[T]? {
        if start >= 0 and end <= self.cap and start <= end {
            .Some(Slice(pointer: self.ptr.offset(by: start), count: end - start))
        } else {
            .None
        }
    }
}
