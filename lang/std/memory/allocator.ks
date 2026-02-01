// Allocator protocol and implementations

module std.memory

import std.result.(Optional)
import std.memory.(Layout, RawPointer)
import std.ffi.(malloc, free, realloc)

/// Protocol defining the memory allocation interface.
/// Allocators manage raw memory allocation, deallocation, and reallocation.
public protocol Allocator {
    /// Allocates memory matching the given layout.
    /// Returns None on allocation failure.
    mutating func allocate(layout: Layout) -> RawPointer?

    /// Deallocates memory previously allocated with this allocator.
    /// The layout must match the layout used for allocation.
    mutating func deallocate(ptr: RawPointer, layout: Layout)

    /// Reallocates memory to a new size.
    /// Returns None on reallocation failure (original memory unchanged).
    mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer?
}

/// System allocator that wraps malloc/free from libc.
/// This is the default allocator used throughout the standard library.
public struct SystemAllocator: Allocator {
    /// Creates a new system allocator instance.
    public init() {}

    /// Allocates memory using malloc.
    /// Note: Alignment beyond natural alignment is not guaranteed.
    public mutating func allocate(layout: Layout) -> RawPointer? {
        let ptr = malloc(layout.size.raw);
        if lang.ptr_is_null(ptr) {
            .None
        } else {
            .Some(RawPointer(raw: ptr))
        }
    }

    /// Deallocates memory using free.
    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        free(ptr.raw)
    }

    /// Reallocates memory using realloc.
    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer? {
        let newPtr = realloc(ptr.raw, newLayout.size.raw);
        if lang.ptr_is_null(newPtr) {
            .None
        } else {
            .Some(RawPointer(raw: newPtr))
        }
    }
}

/// Type alias for the global allocator.
/// Can be customized per project for different allocation strategies.
public type GlobalAllocator = SystemAllocator

// Note: ArenaAllocator and PoolAllocator require Buffer (Phase 14)
// They will be added after Buffer is implemented
