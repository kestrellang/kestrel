// Buffer[T] - owning contiguous memory region

module std.memory

import std.result.(Optional)
import std.core.(Cloneable, Int, Bool)
import std.ops.(NonCopyable)
import std.ffi.(memcpy, memmove, memset)

public struct Buffer[T, A]: NonCopyable where A: Allocator {
    private var ptr: Pointer[T]
    private var cap: Int
    private var allocator: A

    // Allocate buffer with capacity
    public init(capacity: Int) {
        self.allocator = GlobalAllocator();
        let layout = Layout.array[T](count: capacity);
        match self.allocator.allocate(layout: layout) {
            .Some(rawPtr) => {
                self.ptr = rawPtr.cast[T]();
                self.cap = capacity;
            },
            .None => lang.panic("Buffer allocation failed")
        }
    }

    public init(capacity: Int, allocator: A) {
        self.allocator = allocator;
        let layout = Layout.array[T](count: capacity);
        match self.allocator.allocate(layout: layout) {
            .Some(rawPtr) => {
                self.ptr = rawPtr.cast[T]();
                self.cap = capacity;
            },
            .None => lang.panic("Buffer allocation failed")
        }
    }

    // Create from existing pointer (non-owning view)
    private init(pointer: Pointer[T], capacity: Int, allocator: A) {
        self.ptr = pointer;
        self.cap = capacity;
        self.allocator = allocator;
    }

    deinit {
        let layout = Layout.array[T](count: self.cap);
        self.allocator.deallocate(ptr: self.ptr.asRaw(), layout: layout)
    }

    public var capacity: Int { self.cap }

    public var pointer: Pointer[T] { self.ptr }

    // Unchecked access - get element at index without bounds checking
    public func buffer(unchecked index: Int) -> T {
        self.ptr.offset(by: index).read()
    }

    // Unchecked set - set element at index without bounds checking
    public mutating func setBuffer(unchecked index: Int, value: T) {
        self.ptr.offset(by: index).write(value)
    }

    // Safe access - returns Optional, bounds checked
    public subscript(safe index: Int) -> Optional[T] {
        get {
            if index >= 0 and index < self.cap {
                .Some(self.ptr.offset(by: index).read())
            } else {
                .None
            }
        }
        set {
            if index >= 0 and index < self.cap {
                if let value = newValue {
                    self.ptr.offset(by: index).write(value)
                }
            }
        }
    }

    // Wrapping access - indices wrap around
    public subscript(wrapping index: Int) -> T {
        get {
            let wrapped = ((index % self.cap) + self.cap) % self.cap;
            self.ptr.offset(by: wrapped).read()
        }
        set {
            let wrapped = ((index % self.cap) + self.cap) % self.cap;
            self.ptr.offset(by: wrapped).write(newValue)
        }
    }

    // Unchecked access - no bounds check
    public subscript(unchecked index: Int) -> T {
        get { self.ptr.offset(by: index).read() }
        set { self.ptr.offset(by: index).write(newValue) }
    }

    // Bulk operations
    public func fill(with value: T) {
        /* for i in 0..<self.cap {
            self.ptr.offset(by: i).write(value.clone())
        } */
    }

    public func copy(from source: Buffer[T, A], count: Int) {
        let copyCount = if count < source.cap { count } else { source.cap };
        let copyCount = if copyCount < self.cap { copyCount } else { self.cap };
        memcpy(self.ptr.asRaw().raw, source.ptr.asRaw().raw, copyCount * Int(lang.sizeof[T]()));
    }

    public func move(from source: Buffer[T, A], count: Int) {
        let moveCount = if count < source.cap { count } else { source.cap };
        let moveCount = if moveCount < self.cap { moveCount } else { self.cap };
        memmove(self.ptr.asRaw().raw, source.ptr.asRaw().raw, moveCount * Int(lang.sizeof[T]()));
    }

    public func zeroFill() {
        memset(self.ptr.asRaw().raw, 0, self.cap * Int(lang.sizeof[T]()));
    }

    // Resizing
    public mutating func resize(to newCapacity: Int) {
        let oldLayout = Layout.array[T](count: self.cap);
        let newLayout = Layout.array[T](count: newCapacity);

        match self.allocator.reallocate(ptr: self.ptr.asRaw(), oldLayout: oldLayout, newLayout: newLayout) {
            .Some(newPtr) => {
                self.ptr = newPtr.cast[T]();
                self.cap = newCapacity
            },
            .None => lang.panic("Buffer resize failed")
        }
    }

    // Get slice
    public func asSlice() -> Slice[T] {
        Slice(pointer: self.ptr, count: self.cap)
    }

    public func slice(from start: Int, to end: Int) -> Optional[Slice[T]] {
        if start >= 0 and end <= self.cap and start <= end {
            .Some(Slice(pointer: self.ptr.offset(by: start), count: end - start))
        } else {
            .None
        }
    }
}

// ArcBox[T] - reference-counted box for COW types
public struct ArcBox[T] {
    private var ptr: Pointer[ArcBoxStorage[T]]

    struct ArcBoxStorage[T1] {
        var refCount: Int  // Should be atomic
        var value: T1
    }

    public init(value: T) {
        let layout = Layout.of[ArcBoxStorage[T]]();
        let allocator = GlobalAllocator();
        match allocator.allocate(layout: layout) {
            .Some(rawPtr) => {
                self.ptr = rawPtr.cast[ArcBoxStorage[T]]();
                self.ptr.pointee = ArcBoxStorage(refCount: 1, value: value)
            },
            .None => lang.panic("ArcBox allocation failed")
        }
    }

    public var value: /*ref*/ T {
        self.ptr.pointee.value
    }

    public func isUnique() -> Bool {
        self.ptr.pointee.refCount == 1
    }

    public func clone() -> ArcBox[T] {
        lang.atomic_add(self.ptr.pointee.refCount, 1);
        ArcBox(ptr: self.ptr)
    }

    public func deepClone() -> ArcBox[T] where T: Cloneable {
        ArcBox(value: self.ptr.pointee.value.clone())
    }

    private func release() {
        if lang.atomic_sub(self.ptr.pointee.refCount, 1) == 1 {
            // Last reference, deallocate
            let layout = Layout.of[ArcBoxStorage[T]]();
            GlobalAllocator().deallocate(ptr: self.ptr.asRaw(), layout: layout)
        }
    }

    deinit {
        self.release()
    }
}
