// Buffer[T, A] - owning contiguous memory region

module std.memory

import std.result.(Optional)
import std.core.(Bool, NonCopyable)
import std.num.(Int64)
import std.memory.(Layout, Pointer, Slice, RawPointer, Allocator, GlobalAllocator)
import std.ffi.(memcpy, memmove, memset)

public struct Buffer[T, A]: NonCopyable where A: Allocator {
    private var ptr: Pointer[T]
    private var cap: Int64
    private var allocator: A

    // Allocate buffer with capacity using provided allocator
    public init(capacity: Int64, allocator: A) {
        self.allocator = allocator;
        self.cap = capacity;
        let layout = Layout.array[T](capacity);
        let result = self.allocator.allocate(layout);
        if result.isSome() {
            self.ptr = result.unwrap().cast[T]()
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

    deinit {
        let layout = Layout.array[T](self.cap);
        self.allocator.deallocate(self.ptr.asRaw(), layout)
    }

    public var capacity: Int64 { self.cap }

    public var pointer: Pointer[T] { self.ptr }

    // Unchecked read - get element at index without bounds checking
    public func read(unchecked index: Int64) -> T {
        self.ptr.offset(by: index).read()
    }

    // Unchecked write - set element at index without bounds checking
    public func write(unchecked index: Int64, value: T) {
        self.ptr.offset(by: index).write(value)
    }

    // Safe read - returns Optional, bounds checked
    public func read(at index: Int64) -> Optional[T] {
        if index >= 0 and index < self.cap {
            .Some(self.ptr.offset(by: index).read())
        } else {
            .None
        }
    }

    // Safe write - bounds checked, returns success
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

    // Resizing
    public mutating func resize(to newCapacity: Int64) {
        let oldLayout = Layout.array[T](self.cap);
        let newLayout = Layout.array[T](newCapacity);

        let result = self.allocator.reallocate(self.ptr.asRaw(), oldLayout, newLayout);
        if result.isSome() {
            self.ptr = result.unwrap().cast[T]();
            self.cap = newCapacity
        } else {
            lang.panic("Buffer resize failed")
        }
    }

    // Get slice view
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr, count: self.cap)
    }

    public func slice(from start: Int64, to end: Int64) -> Optional[Slice[T]] {
        if start >= 0 and end <= self.cap and start <= end {
            .Some(Slice(pointer: self.ptr.offset(by: start), count: end - start))
        } else {
            .None
        }
    }
}
