// ArenaAllocator - bump allocator that frees everything at once

module std.memory

import std.core.(Bool, fatalError)
import std.numeric.(Int64, UInt64)
import std.result.(Optional)
import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator)
import std.ffi.(memcpy)

/// A bump allocator: hands out memory from big chunks by moving a cursor,
/// and gives it all back at once with `reset()` or `release()`.
///
/// Allocating is a pointer bump; `deallocate` does nothing except take
/// back the most recent allocation, and `reallocate` grows the most
/// recent allocation in place when it fits. That suits many short-lived
/// objects that die together: a tree built and thrown away, per-request
/// scratch arrays, a parser's nodes.
///
/// `ArenaAllocator` is a handle: copies share one arena. That is what lets
/// a collection keep a copy of its allocator (`Array[T, ArenaAllocator]`).
/// Nothing frees the arena automatically; call `release()` once nothing
/// allocated from it is used any more.
///
/// # Examples
///
/// ```
/// var arena = ArenaAllocator();
/// for _ in 0..<1000 {
///     var xs = Array[Int64, ArenaAllocator](allocator: arena);
///     for i in 0..<100 { xs.append(i); }
///     // ... use xs; it must be gone before the reset
///     arena.reset();   // keeps the memory for the next round
/// }
/// arena.release();
/// ```
///
/// # Representation
///
/// One pointer to a heap `ArenaState`: the current chunk, the cursor and
/// end within it, and the size of the next chunk. Each chunk starts with
/// a header linking it to the previous one.
///
/// # Memory Model
///
/// Not thread-safe. Memory from the arena is valid until the next `reset()`
/// or `release()`; using it after that is a use-after-free, and values
/// still holding arena memory then must not be dropped afterwards either.
public struct ArenaAllocator: Allocator {
    private var state: Pointer[ArenaState]

    /// @name Default
    /// An empty arena whose first chunk is 64 KiB.
    public init() {
        self.init(chunkSize: 65536)
    }

    /// @name Chunk Size
    /// An empty arena whose first chunk holds `chunkSize` bytes. Later
    /// chunks double, up to 16 MiB.
    public init(chunkSize chunkSize: Int64) {
        var system = SystemAllocator();
        if let .Some(raw) = system.allocate(Layout.of[ArenaState]()) {
            self.state = raw.cast[ArenaState]();
            unsafe self.state.write(ArenaState(nextChunkSize: if chunkSize < 64 { 64 } else { chunkSize }));
        } else {
            fatalError("ArenaAllocator: out of memory")
        }
    }

    public mutating func allocate(layout: Layout) -> RawPointer? {
        if layout.size < 0 or not layout.alignment.isPowerOfTwo { return .None; }
        let s = unsafe &mutating self.state.mutatingValue;
        let start = alignUp(s.cursor, layout.alignment);
        if start + layout.size <= s.end {
            s.last = start;
            s.cursor = start + layout.size;
            return .Some(pointerAt(start));
        }
        if not s.addChunk(layout.size + layout.alignment) { return .None; }
        let fresh = alignUp(s.cursor, layout.alignment);
        s.last = fresh;
        s.cursor = fresh + layout.size;
        .Some(pointerAt(fresh))
    }

    /// Takes the memory back only when `ptr` is the most recent
    /// allocation; otherwise it waits for `reset()`.
    public unsafe mutating func deallocate(ptr: RawPointer, layout: Layout) {
        let s = unsafe &mutating self.state.mutatingValue;
        if addressOf(ptr) == s.last {
            s.cursor = s.last;
            s.last = 0
        }
    }

    /// Grows or shrinks the most recent allocation in place when it fits;
    /// otherwise allocates afresh and copies.
    public unsafe mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer? {
        let s = unsafe &mutating self.state.mutatingValue;
        let addr = addressOf(ptr);
        if addr == s.last and addr & (newLayout.alignment - 1) == 0 and addr + newLayout.size <= s.end {
            s.cursor = addr + newLayout.size;
            return .Some(ptr);
        }
        let fresh = match self.allocate(newLayout) {
            .Some(p) => p,
            .None => return .None,
        };
        let n = if oldLayout.size < newLayout.size { oldLayout.size } else { newLayout.size };
        if n > 0 { let _ = unsafe memcpy(fresh, ptr, n); }
        .Some(fresh)
    }

    /// Frees every chunk but the newest (the largest) and starts over in
    /// it. Everything allocated before is invalid afterwards.
    public mutating func reset() {
        let s = unsafe &mutating self.state.mutatingValue;
        if s.chunk == 0 { return; }
        let newest = chunkHeader(s.chunk);
        s.freeChunks(newest.previous);
        unsafe pointerAt(s.chunk).cast[ChunkHeader]().write(ChunkHeader(previous: 0, size: newest.size));
        s.cursor = s.chunk + chunkHeaderSize;
        s.last = 0
    }

    /// Frees every chunk and the arena itself. Every copy of this handle
    /// is invalid afterwards.
    public consuming func release() {
        let s = unsafe &mutating self.state.mutatingValue;
        s.freeChunks(s.chunk);
        var system = SystemAllocator();
        unsafe system.deallocate(self.state.asRaw(), Layout.of[ArenaState]())
    }

    /// Bytes reserved from the system, across all chunks.
    public var reservedBytes: Int64 {
        var total: Int64 = 0;
        var c: Int64 = unsafe self.state.read().chunk;
        while c != 0 {
            let header = chunkHeader(c);
            total = total + header.size;
            c = header.previous
        };
        total
    }
}

/// The arena behind an `ArenaAllocator` handle. Addresses are `Int64`
/// (user-space addresses are below 2^63) so the bump arithmetic mixes
/// with `Layout` sizes without conversions.
struct ArenaState {
    /// Start of the newest chunk, 0 for none.
    var chunk: Int64
    /// Next free byte in the newest chunk.
    var cursor: Int64
    /// One past the newest chunk's last byte.
    var end: Int64
    /// Start of the most recent allocation, 0 when it was given back.
    var last: Int64
    /// Size of the next chunk to reserve.
    var nextChunkSize: Int64

    init(nextChunkSize nextChunkSize: Int64) {
        self.chunk = 0;
        self.cursor = 0;
        self.end = 0;
        self.last = 0;
        self.nextChunkSize = nextChunkSize;
    }

    /// Reserves a chunk with room for at least `need` bytes after its
    /// header and makes it current. `false` when the system is out of
    /// memory.
    mutating func addChunk(need: Int64) -> Bool {
        var size = self.nextChunkSize;
        while size - chunkHeaderSize < need { size = size * 2 }
        var system = SystemAllocator();
        let raw = match system.allocate(Layout(size: size, alignment: 16)) {
            .Some(p) => p,
            .None => return false,
        };
        unsafe raw.cast[ChunkHeader]().write(ChunkHeader(previous: self.chunk, size: size));
        self.chunk = addressOf(raw);
        self.cursor = self.chunk + chunkHeaderSize;
        self.end = self.chunk + size;
        if self.nextChunkSize < 16777216 { self.nextChunkSize = self.nextChunkSize * 2 }
        true
    }

    /// Frees the chunk at `start` and every chunk before it.
    mutating func freeChunks(start: Int64) {
        var system = SystemAllocator();
        var c = start;
        while c != 0 {
            let header = chunkHeader(c);
            unsafe system.deallocate(pointerAt(c), Layout(size: header.size, alignment: 16));
            c = header.previous
        }
    }
}

/// The start of every chunk: the chunk before it (0 for none) and this
/// chunk's size.
struct ChunkHeader {
    var previous: Int64
    var size: Int64
}

/// Bytes before a chunk's first allocation (the header, 16-aligned).
let chunkHeaderSize: Int64 = 16

fileprivate func chunkHeader(chunk: Int64) -> ChunkHeader {
    unsafe pointerAt(chunk).cast[ChunkHeader]().read()
}

fileprivate func addressOf(ptr: RawPointer) -> Int64 {
    Int64(truncating: ptr.address)
}

fileprivate func pointerAt(addr: Int64) -> RawPointer {
    RawPointer(address: UInt64(truncating: addr))
}

/// `addr` rounded up to a multiple of `alignment` (a power of two).
fileprivate func alignUp(addr: Int64, alignment: Int64) -> Int64 {
    let mask = alignment - 1;
    (addr + mask) & mask.bitwiseNot()
}
