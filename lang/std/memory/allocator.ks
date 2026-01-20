// Allocator protocol and implementations

module std.memory

import std.result.(Optional)
import std.memory.(Layout, RawPointer)
import std.ffi.(malloc, free, realloc)

// Allocator protocol - defines memory allocation interface
public protocol Allocator {
    mutating func allocate(layout: Layout) -> Optional[RawPointer]
    mutating func deallocate(ptr: RawPointer, layout: Layout)
    mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer]
}

// SystemAllocator - wrapper around system malloc/free
public struct SystemAllocator: Allocator {
    public init() {}

    public mutating func allocate(layout: Layout) -> Optional[RawPointer] {
        // Note: malloc doesn't guarantee alignment beyond natural alignment
        // For stricter alignment, use posix_memalign
        let ptr = malloc(layout.size.raw);
        if lang.ptr_is_null(ptr) {
            .None
        } else {
            .Some(RawPointer(raw: ptr))
        }
    }

    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        free(ptr.raw)
    }

    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
        let newPtr = realloc(ptr.raw, newLayout.size.raw);
        if lang.ptr_is_null(newPtr) {
            .None
        } else {
            .Some(RawPointer(raw: newPtr))
        }
    }
}

// Global allocator type alias - can be customized per project
public type GlobalAllocator = SystemAllocator

// Note: ArenaAllocator and PoolAllocator require Buffer (Phase 14)
// They will be added after Buffer is implemented
