// Allocator protocol and implementations

module std.memory

public protocol Allocator {
    func allocate(layout: Layout) -> Optional[RawPointer]
    func deallocate(ptr: RawPointer, layout: Layout)
    func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer]
}

// Default reallocation implementation
extend Allocator {
    public func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
        // Allocate new block
        if let newPtr = self.allocate(layout: newLayout) {
            // Copy old data
            let copySize = if oldLayout.size < newLayout.size { oldLayout.size } else { newLayout.size };
            lang.memcpy(newPtr.raw, ptr.raw, copySize);
            // Free old block
            self.deallocate(ptr: ptr, layout: oldLayout);
            return .Some(newPtr)
        }
        .None
    }
}

// SystemAllocator - wrapper around system malloc/free
public struct SystemAllocator: Allocator {
    public init() {}

    public func allocate(layout: Layout) -> Optional[RawPointer] {
        let ptr = lang.alloc(layout.size, layout.alignment);
        if lang.ptr_is_null(ptr) {
            .None
        } else {
            .Some(RawPointer(raw: ptr))
        }
    }

    public func deallocate(ptr: RawPointer, layout: Layout) {
        lang.dealloc(ptr.raw, layout.size, layout.alignment)
    }

    public func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> Optional[RawPointer] {
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

    public func allocate(layout: Layout) -> Optional[RawPointer] {
        // Align offset
        let alignedOffset = (self.offset + layout.alignment - 1).bitwiseAnd((layout.alignment - 1).bitwiseNot());

        if alignedOffset + layout.size > self.buffer.capacity {
            return .None
        }

        let ptr = self.buffer.pointer.offset(by: alignedOffset).asRaw();
        self.offset = alignedOffset + layout.size;
        .Some(ptr)
    }

    public func deallocate(ptr: RawPointer, layout: Layout) {
        // No-op for arena - memory freed all at once
    }

    public func reset() {
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

    public func allocate(layout: Layout) -> Optional[RawPointer] {
        // Pool allocator only works for its specific type size
        if layout.size != Layout.of[T]().size {
            return .None
        }

        if let node = self.freeList {
            self.freeList = node.pointee.next;
            self.allocated = self.allocated + 1;
            .Some(node.asRaw())
        } else {
            .None
        }
    }

    public func deallocate(ptr: RawPointer, layout: Layout) {
        let node = ptr.cast[FreeNode]();
        node.pointee = FreeNode(next: self.freeList);
        self.freeList = .Some(node);
        self.allocated = self.allocated - 1
    }

    public var count: Int {
        self.allocated
    }

    public var capacity: Int {
        self.buffer.capacity
    }
}
