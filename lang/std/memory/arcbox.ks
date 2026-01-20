// ArcBox[T] - reference-counted box for COW (copy-on-write) semantics

module std.memory

import std.core.(Bool, Cloneable, Copyable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator)

// Storage for ArcBox - holds refcount and value
struct ArcBoxStorage[T] {
    var refCount: Int64  // TODO: Should be atomic
    var value: T
}

// ArcBox[T] - reference-counted heap allocation
// Used for implementing copy-on-write semantics in types like String and Array
public struct ArcBox[T] {
    private var ptr: Pointer[ArcBoxStorage[T]]

    // Create new ArcBox with initial value
    public init(value: T) {
        let layout: Layout = Layout.of[ArcBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: Optional[RawPointer] = allocator.allocate(layout);
        if result.isSome() {
            self.ptr = result.unwrap().cast[ArcBoxStorage[T]]();
            self.ptr.write(ArcBoxStorage(refCount: Int64(intLiteral: 1), value: value));
        } else {
            lang.panic("ArcBox allocation failed")
        }
    }

    // Private init for clone (shares storage)
    private init(inner inner: Pointer[ArcBoxStorage[T]]) {
        self.ptr = inner;
    }

    // Access the stored value
    public func getValue() -> T {
        self.ptr.read().value
    }

    // Check if this is the only reference
    public func isUnique() -> Bool {
        self.ptr.read().refCount == Int64(intLiteral: 1)
    }

    // Get current reference count
    public func refCount() -> Int64 {
        self.ptr.read().refCount
    }

    // Shallow clone - increments refcount, shares storage
    public func clone() -> ArcBox[T] {
        // TODO: Should use atomic increment
        var storage = self.ptr.read();
        storage.refCount = storage.refCount + Int64(intLiteral: 1);
        self.ptr.write(storage);
        ArcBox(inner: self.ptr)
    }

    // Deep clone - creates new storage with cloned value
    public func deepClone() -> ArcBox[T] where T: Cloneable {
        ArcBox(self.ptr.read().value.clone())
    }

    // Release reference (called by deinit)
    private func release() {
        // TODO: Should use atomic decrement
        var storage = self.ptr.read();
        storage.refCount = storage.refCount - Int64(intLiteral: 1);

        if storage.refCount == Int64(intLiteral: 0) {
            // Last reference, deallocate
            let layout = Layout.of[ArcBoxStorage[T]]();
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        } else {
            self.ptr.write(storage)
        }
    }

    deinit {
        self.release()
    }
}
