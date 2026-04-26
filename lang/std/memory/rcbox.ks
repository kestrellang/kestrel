// RcBox[T] - reference-counted box for COW (copy-on-write) semantics

module std.memory

import std.core.(Bool, Cloneable, Copyable)
import std.num.(Int64)
import std.result.(Optional)
import std.memory.(Layout, Pointer, RawPointer, Allocator, SystemAllocator)

// Storage block backing an RcBox: the refcount lives next to the value,
// in a single allocation, so clones bump a counter rather than copying T.
struct RcBoxStorage[T] {
    var refCount: Int64  // TODO: Should be atomic
    var value: T
}

/// Heap allocation with a strong-reference count, used as the underlying
/// storage for the stdlib's copy-on-write types.
///
/// `String`, `Array`, and `Dictionary` all wrap an `RcBox` so that a
/// plain assignment shares storage and only the first mutating call pays
/// for a deep copy. Reach for `RcBox` directly when building a similar
/// COW type; for plain shared ownership without mutation prefer a more
/// purpose-built container.
///
/// # Examples
///
/// ```
/// let a = RcBox(value: [1, 2, 3]);
/// let b = a.clone();          // shares storage; refCount == 2
/// if b.isUnique() { ... } else { let c = b.deepClone(); /* ... */ }
/// ```
///
/// # Representation
///
/// One `Pointer[RcBoxStorage[T]]`. The pointed-to block holds an `Int64`
/// refcount followed by the `T` value, allocated via `SystemAllocator`.
///
/// # Memory Model
///
/// Reference-counted, non-atomic (today — see TODOs). `clone()` increments
/// the count and shares storage; `deinit` decrements and frees on zero.
/// `deepClone()` allocates a fresh `RcBox` carrying a copied value.
///
/// # Guarantees
///
/// - `isUnique()` returning `true` means in-place mutation is safe; this is
///   how COW types decide whether to copy.
/// - The refcount is currently **not** atomic, so `RcBox` is not safe to
///   share across threads.
public struct RcBox[T] {
    private var ptr: Pointer[RcBoxStorage[T]]

    /// @name From Value
    /// Allocates fresh storage holding `value` with refcount 1. Panics if
    /// the underlying `SystemAllocator` returns `.None`.
    ///
    /// # Errors
    ///
    /// Panics with `"RcBox allocation failed"` on allocation failure.
    public init(value: T) {
        let layout: Layout = Layout.of[RcBoxStorage[T]]();
        var allocator: SystemAllocator = SystemAllocator();
        let result: RawPointer? = allocator.allocate(layout);
        if let .Some(rawPtr) = result {
            self.ptr = rawPtr.cast[RcBoxStorage[T]]();
            self.ptr.write(RcBoxStorage(refCount: Int64(intLiteral: 1), value: value));
        } else {
            lang.panic("RcBox allocation failed")
        }
    }

    // Private init used by clone(): adopts an existing storage block
    // (which has already been refcount-bumped) without allocating.
    private init(inner inner: Pointer[RcBoxStorage[T]]) {
        self.ptr = inner;
    }

    /// Reads the wrapped value out of storage. Returns a copy — the
    /// underlying `T` is read through a pointer, so callers see a
    /// snapshot, not a live reference.
    public func getValue() -> T {
        self.ptr.read().value
    }

    /// Overwrites the wrapped value in place. Safe only when this is the
    /// unique owner (`isUnique() == true`); otherwise other clones see the
    /// new value, defeating COW. The COW types check `isUnique` before
    /// calling this and `deepClone` otherwise.
    public func setValue(value: T) {
        var storage = self.ptr.read();
        storage.value = value;
        self.ptr.write(storage);
    }

    /// Returns `true` when no other clone is sharing storage. The litmus
    /// test for "safe to mutate in place" in COW collections.
    public func isUnique() -> Bool {
        self.ptr.read().refCount == Int64(intLiteral: 1)
    }

    /// Current strong reference count. Mostly useful for tests and
    /// diagnostics; production COW logic should branch on `isUnique`.
    public func refCount() -> Int64 {
        self.ptr.read().refCount
    }

    /// Bumps the refcount and returns a second `RcBox` pointing at the
    /// same storage. The receiver and the returned box now both reference
    /// the value; the next mutation should test `isUnique`.
    public func clone() -> RcBox[T] {
        // TODO: Should use atomic increment
        var storage = self.ptr.read();
        storage.refCount = storage.refCount + Int64(intLiteral: 1);
        self.ptr.write(storage);
        RcBox(inner: self.ptr)
    }

    /// Allocates fresh storage with a copy of the value. Used by COW
    /// types when `isUnique()` returns `false` — splits off a private
    /// copy so the caller can mutate without affecting other clones.
    public func deepClone() -> RcBox[T] {
        RcBox(self.ptr.read().value)
    }

    // Drop one reference; deallocate storage when the count hits zero.
    // Called from deinit; not exposed publicly.
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

    /// Decrements the refcount; deallocates storage when it reaches zero.
    deinit {
        self.release()
    }
}
