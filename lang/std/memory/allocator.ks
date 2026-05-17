// Allocator protocol and implementations

module std.memory

import std.result.(Optional)
import std.memory.(Layout, RawPointer)
import std.ffi.(malloc, free, realloc)

/// Protocol for raw-memory allocators.
///
/// `Allocator` is the indirection collections use so they can be parameterised
/// over allocation strategy (e.g. `Array[T, A]`, `Buffer[T, A]`, custom
/// arenas). All three methods are `mutating` so stateful allocators (arenas,
/// pools) can update their bookkeeping; stateless wrappers around `malloc`
/// don't need to.
///
/// # Examples
///
/// ```
/// var alloc = SystemAllocator();
/// if let .Some(p) = alloc.allocate(Layout.of[Int64]()) {
///     // ... use p ...
///     alloc.deallocate(p, Layout.of[Int64]())
/// }
/// ```
public protocol Allocator {
    /// Returns a pointer to a fresh region matching `layout`, or `.None`
    /// when allocation fails. Returned memory is uninitialised.
    mutating func allocate(layout: Layout) -> RawPointer?

    /// Releases memory previously returned by `allocate` / `reallocate`.
    /// `layout` must match the layout used to obtain the pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must have been produced by this allocator (or a clone of it)
    /// for `layout`. Mismatching the layout, double-freeing, or freeing a
    /// pointer from another allocator is undefined behavior.
    mutating func deallocate(ptr: RawPointer, layout: Layout)

    /// Resizes the allocation at `ptr` from `oldLayout` to `newLayout`.
    /// On failure the original allocation is left intact and `.None` is
    /// returned. On success the old pointer must not be reused — use the
    /// returned pointer instead.
    mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer?
}

/// `Allocator` backed by libc `malloc`/`free`/`realloc`. Used as the
/// default `GlobalAllocator` and by every collection that doesn't pick a
/// custom allocator.
///
/// # Memory Model
///
/// Stateless: the struct holds no fields. All bookkeeping lives in libc's
/// heap. Cloning or copying the allocator has no effect on the heap state.
public struct SystemAllocator: Allocator {
    /// @name Default
    /// Builds a stateless system allocator. No heap interaction occurs here.
    public init() {}

    /// Calls `malloc(layout.size)`. Alignment beyond `malloc`'s natural
    /// alignment (typically 16) is **not** honoured — types that need
    /// larger alignment should use a different allocator.
    public mutating func allocate(layout: Layout) -> RawPointer? {
        let ptr = malloc(layout.size);
        if ptr.isNull {
            .None
        } else {
            .Some(ptr)
        }
    }

    /// Calls `free(ptr)`. The `layout` argument is ignored — kept for
    /// protocol conformance; allocators that need it (arenas) use it.
    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        free(ptr)
    }

    /// Calls `realloc(ptr, newLayout.size)`. As with `allocate`, only
    /// `malloc`-natural alignment is guaranteed.
    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer? {
        let newPtr = realloc(ptr, newLayout.size);
        if newPtr.isNull {
            .None
        } else {
            .Some(newPtr)
        }
    }
}

/// Project-wide default allocator, aliased to `SystemAllocator`. Override
/// at the project level if a global custom allocator is needed.
public type GlobalAllocator = SystemAllocator

// Note: ArenaAllocator and PoolAllocator require Buffer (Phase 14)
// They will be added after Buffer is implemented
