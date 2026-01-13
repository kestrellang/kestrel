// Allocator protocol and implementations

module std.memory

import std.result.(Optional)
import std.core.(Int, UInt8)

public protocol Allocator {
    mutating func allocate(layout: Layout) -> Optional[RawPointer]
    mutating func deallocate(ptr: RawPointer, layout: Layout)
    mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer]
}

// TODO: Protocol extensions not yet supported
// Default reallocation implementation
// extend Allocator {
//     public func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
//         // Allocate new block
//         if let newPtr = self.allocate(layout: newLayout) {
//             // Copy old data
//             let copySize = if oldLayout.size < newLayout.size { oldLayout.size } else { newLayout.size };
//             lang.memcpy(newPtr.raw, ptr.raw, copySize);
//             // Free old block
//             self.deallocate(ptr: ptr, layout: oldLayout);
//             return .Some(newPtr)
//         }
//         .None
//     }
// }

// SystemAllocator - wrapper around system malloc/free
public struct SystemAllocator: Allocator {
    public init() {}

    public mutating func allocate(layout: Layout) -> Optional[RawPointer] {
        let ptr = lang.alloc(layout.size, layout.alignment);
        if lang.ptr_is_null(ptr) {
            .None
        } else {
            .Some(RawPointer(raw: ptr))
        }
    }

    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        lang.dealloc(ptr.raw, layout.size, layout.alignment)
    }

    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
        let newPtr = lang.realloc(ptr.raw, oldLayout.size, newLayout.size, newLayout.alignment);
        if lang.ptr_is_null(newPtr) {
            .None
        } else {
            .Some(RawPointer(raw: newPtr))
        }
    }
}

// Global allocator type alias - can be customized per project
public type GlobalAllocator = SystemAllocator

// ArenaAllocator - bump allocation with bulk deallocation
public struct ArenaAllocator: Allocator {
    private var buffer: Buffer[UInt8, SystemAllocator]
    private var offset: Int

    public init(capacity: Int) {
        self.buffer = Buffer(capacity: capacity);
        self.offset = 0
    }

    public mutating func allocate(layout: Layout) -> Optional[RawPointer] {
        // Align offset
        let alignedOffset = (self.offset + layout.alignment - 1).bitwiseAnd((layout.alignment - 1).bitwiseNot());

        if alignedOffset + layout.size > self.buffer.capacity {
            return .None
        }

        let ptr = self.buffer.pointer.offset(by: alignedOffset).asRaw();
        self.offset = alignedOffset + layout.size;
        .Some(ptr)
    }

    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        // No-op for arena - memory freed all at once
    }

    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
        // Arena allocator cannot reuse old blocks
        .None
    }

    public mutating func reset() {
        self.offset = 0
    }

    public var bytesUsed: Int {
        self.offset
    }

    public var bytesRemaining: Int {
        self.buffer.capacity - self.offset
    }
}

// PoolAllocator - fixed-size block allocation
public struct PoolAllocator[T]: Allocator {
    private var buffer: Buffer[T, SystemAllocator]
    private var freeList: Optional[Pointer[FreeNode]]
    private var allocated: Int

    struct FreeNode {
        var next: Optional[Pointer[FreeNode]]
    }

    public init(capacity: Int) {
        self.buffer = Buffer(capacity: capacity);
        self.freeList = .None;
        self.allocated = 0;

        // Initialize free list
        /* for i in (0..<capacity).reversed() {
            let node = self.buffer.pointer.offset(by: i).cast[FreeNode]()
            node.pointee = FreeNode(next: self.freeList)
            self.freeList = .Some(node)
        } */
    }

    public mutating func allocate(layout: Layout) -> Optional[RawPointer] {
        // Pool allocator only works for its specific type size
        if layout.size != Layout.of[T]().size {
            return .None
        }

        if let .Some(node) = self.freeList {
            self.freeList = node.pointee.next;
            self.allocated = self.allocated + 1;
            .Some(node.asRaw())
        } else {
            .None
        }
    }

    public mutating func deallocate(ptr: RawPointer, layout: Layout) {
        let node = ptr.cast[FreeNode]();
        node.pointee = FreeNode(next: self.freeList);
        self.freeList = .Some(node);
        self.allocated = self.allocated - 1
    }

    public mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
        // Pool allocator cannot resize blocks
        .None
    }

    public var count: Int {
        self.allocated
    }

    public var capacity: Int {
        self.buffer.capacity
    }
}
