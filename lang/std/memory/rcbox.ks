// RcBox[T] - reference-counted box for COW (copy-on-write) semantics

module std.memory

import std.core.(Bool, Cloneable, Copyable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator)

// Storage for RcBox - holds refcount and value
struct RcBoxStorage[T] {
    var refCount: Int64  // TODO: Should be atomic
    var value: T
}

/// A reference-counted heap allocation for implementing copy-on-write semantics.
/// Used internally by types like String, Array, and Dictionary to enable
/// efficient copies that share storage until mutation occurs.
public struct RcBox[T] {
    private var ptr: Pointer[RcBoxStorage[T]]

    /// Creates a new RcBox containing the given value.
    /// Allocates heap storage with an initial reference count of 1.
    public init(value: T) {
        let layout: Layout = Layout.of[RcBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: Optional[RawPointer] = allocator.allocate(layout);
        if result.isSome() {
            self.ptr = result.unwrap().cast[RcBoxStorage[T]]();
            self.ptr.write(RcBoxStorage(refCount: Int64(intLiteral: 1), value: value));
        } else {
            lang.panic("RcBox allocation failed")
        }
    }

    // Private init for clone (shares storage)
    private init(inner inner: Pointer[RcBoxStorage[T]]) {
        self.ptr = inner;
    }

    /// Returns the stored value.
    public func getValue() -> T {
        self.ptr.read().value
    }

    /// Sets the stored value (for in-place mutation when unique).
    public func setValue(value: T) {
        var storage = self.ptr.read();
        storage.value = value;
        self.ptr.write(storage);
    }

    /// Returns true if this is the only reference to the storage.
    /// Used to determine if mutation requires copying.
    public func isUnique() -> Bool {
        self.ptr.read().refCount == Int64(intLiteral: 1)
    }

    /// Returns the current reference count.
    public func refCount() -> Int64 {
        self.ptr.read().refCount
    }

    /// Creates a shallow clone that shares storage.
    /// Increments the reference count without copying the value.
    public func clone() -> RcBox[T] {
        // TODO: Should use atomic increment
        var storage = self.ptr.read();
        storage.refCount = storage.refCount + Int64(intLiteral: 1);
        self.ptr.write(storage);
        RcBox(inner: self.ptr)
    }

    /// Creates a deep clone with new storage and a cloned value.
    /// Used when mutation is needed on shared storage.
    public func deepClone() -> RcBox[T] where T: Cloneable {
        RcBox(self.ptr.read().value.clone())
    }

    // Release reference (called by deinit)
    private func release() {
        // TODO: Should use atomic decrement
        var storage = self.ptr.read();
        storage.refCount = storage.refCount - Int64(intLiteral: 1);

        if storage.refCount == Int64(intLiteral: 0) {
            // Last reference, deallocate
            let layout = Layout.of[RcBoxStorage[T]]();
            var allocator = SystemAllocator();
            allocator.deallocate(self.ptr.asRaw(), layout)
        } else {
            self.ptr.write(storage)
        }
    }

    /// Destructor: decrements reference count and deallocates when zero.
    deinit {
        self.release()
    }
}
