// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: 1 1 9\n

// `var (xs, n) = f()` binds through a hidden temp. The destructure moves the
// array out of it, so `xs` is the only owner and its first write doesn't
// copy the buffer. A counting allocator sees every allocation: one for the
// array's storage, none for a copy.
module Test

import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator)

struct Counting: Allocator {
    var count: Pointer[Int64]

    mutating func allocate(layout: Layout) -> RawPointer? {
        unsafe self.count.write(unsafe self.count.read() + 1);
        var s = SystemAllocator();
        s.allocate(layout)
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

func make(alloc: Counting) -> (Array[Int64, Counting], Int64) {
    var xs = Array[Int64, Counting](capacity: 8, allocator: alloc);
    xs.append(1);
    (xs, 1)
}

@main
func main() {
    var s = SystemAllocator();
    let raw = match s.allocate(Layout.of[Int64]()) { .Some(p) => p, .None => fatalError("oom") };
    let counter = raw.cast[Int64]();
    unsafe counter.write(0);
    let alloc = Counting(count: counter);
    var (xs, n) = make(alloc);
    let before = unsafe counter.read();
    xs.append(9);
    let after = unsafe counter.read();
    let last = xs(1);
    print("\(after - before + n) \(xs.count - n) \(last)\n");
    unsafe s.deallocate(raw, Layout.of[Int64]());
}
