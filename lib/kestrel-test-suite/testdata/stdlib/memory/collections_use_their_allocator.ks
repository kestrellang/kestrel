// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: deque 6 true\ndictionary 100 true\nset 100 true\nheap 1 true\nbuilder 50 true\nbalanced true\n

// Deque, Dictionary, Set, Heap and ArrayBuilder take an allocator like Array
// does: every buffer they (and their clones) allocate comes from it, and
// every one goes back to it when they drop.
module Test

import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator)
import std.collections.(Deque, Heap, ArrayBuilder, DefaultHasher)

// Counts live allocations through a shared counter.
struct Counting: Allocator {
    var live: Pointer[Int64]

    mutating func allocate(layout: Layout) -> RawPointer? {
        unsafe self.live.write(unsafe self.live.read() + 1);
        var s = SystemAllocator();
        s.allocate(layout)
    }

    unsafe mutating func deallocate(ptr: RawPointer, layout: Layout) {
        unsafe self.live.write(unsafe self.live.read() - 1);
        var s = SystemAllocator();
        unsafe s.deallocate(ptr, layout)
    }

    unsafe mutating func reallocate(ptr: RawPointer, oldLayout: Layout, newLayout: Layout) -> RawPointer? {
        var s = SystemAllocator();
        unsafe s.reallocate(ptr, oldLayout, newLayout)
    }
}

func live(a: Counting) -> Int64 { unsafe a.live.read() }

func useAll(alloc: Counting) {
    var d = Deque[Int64, Counting](allocator: alloc);
    for i in 0..<5 { d.pushBack(i); }
    var d2 = d.clone();
    d2.pushFront(-1);
    print("deque \(d2.count) \(live(alloc) >= 2)\n");

    var m = Dictionary[Int64, Int64, DefaultHasher, Counting](allocator: alloc);
    for i in 0..<100 { m.insert(i, i * i); }
    var m2 = m.clone();
    m2.insert(1000, 1);
    let before = live(alloc);
    let f = m.filter(where: { (k, v) in k < 10 });
    print("dictionary \(m.count) \(live(alloc) > before and f.count == 10)\n");

    var s = Set[Int64, DefaultHasher, Counting](capacity: 16, allocator: alloc);
    for i in 0..<100 { s.insert(i); }
    print("set \(s.count) \(live(alloc) > before)\n");

    var h = Heap[Int64, Counting](allocator: alloc);
    for x in [5, 3, 9, 1, 7] { h.push(x); }
    let top = match h.peek() { .Some(x) => x, .None => -1 };
    print("heap \(top) \(h.allocator.live == alloc.live)\n");

    var b = ArrayBuilder[Int64, Counting](allocator: alloc);
    for i in 0..<50 { b.append(i); }
    let built = b.build();
    print("builder \(built.count) \(built.allocator.live == alloc.live)\n");
}

@main
func main() {
    var sys = SystemAllocator();
    let raw = match sys.allocate(Layout.of[Int64]()) { .Some(p) => p, .None => fatalError("oom") };
    let counter = raw.cast[Int64]();
    unsafe counter.write(0);
    let alloc = Counting(live: counter);
    useAll(alloc);
    print("balanced \(live(alloc) == 0)\n");
    unsafe sys.deallocate(raw, Layout.of[Int64]());
}
