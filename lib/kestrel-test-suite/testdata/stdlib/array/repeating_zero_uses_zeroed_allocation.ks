// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: Int64 0: zeroed\nInt64 5: plain\nPair zero: zeroed\nFloat64 -0.0: plain\nString empty: plain\nvalues ok\n

// `Array(repeating:count:)` asks its allocator for zeroed memory, and writes
// nothing, when the value is copied byte for byte and its bytes are all zero.
// Anything else (non-zero bytes, a value copied by `clone`) gets plain memory
// and a write per slot.
module Test

import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator)

// Records which entry point the last allocation came through.
struct Recording: Allocator {
    var zeroed: Pointer[Bool]

    init() {
        var s = SystemAllocator();
        let raw = match s.allocate(Layout.of[Bool]()) { .Some(p) => p, .None => fatalError("oom") };
        self.zeroed = raw.cast[Bool]();
        unsafe self.zeroed.write(false);
    }

    mutating func allocate(layout: Layout) -> RawPointer? {
        unsafe self.zeroed.write(false);
        var s = SystemAllocator();
        s.allocate(layout)
    }

    mutating func allocateZeroed(layout: Layout) -> RawPointer? {
        unsafe self.zeroed.write(true);
        var s = SystemAllocator();
        s.allocateZeroed(layout)
    }

    unsafe mutating func deallocate(ptr: RawPointer, layout: Layout) {
        var s = SystemAllocator();
        unsafe s.deallocate(ptr, layout)
    }

    unsafe mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer? {
        var s = SystemAllocator();
        unsafe s.reallocate(ptr, oldLayout, newLayout)
    }
}

struct Pair { var a: Int64; var b: Int64 }

func report(name: String, alloc: Recording) {
    let how = if unsafe alloc.zeroed.read() { "zeroed" } else { "plain" };
    print("\(name): \(how)\n");
}

@main
func main() {
    let alloc = Recording();
    let zeros = Array[Int64, Recording](repeating: 0, count: 1000, allocator: alloc);
    report("Int64 0", alloc);
    let fives = Array[Int64, Recording](repeating: 5, count: 1000, allocator: alloc);
    report("Int64 5", alloc);
    let pairs = Array[Pair, Recording](repeating: Pair(a: 0, b: 0), count: 10, allocator: alloc);
    report("Pair zero", alloc);
    let negZeros = Array[Float64, Recording](repeating: -0.0, count: 10, allocator: alloc);
    report("Float64 -0.0", alloc);
    let strings = Array[String, Recording](repeating: "", count: 10, allocator: alloc);
    report("String empty", alloc);

    var ok = zeros(999) == 0 and fives(999) == 5 and pairs(9).b == 0 and strings(9) == "";
    let negBits = negZeros(9).bitPattern;
    ok = ok and negBits != 0;
    print(if ok { "values ok\n" } else { "values wrong\n" });
}
