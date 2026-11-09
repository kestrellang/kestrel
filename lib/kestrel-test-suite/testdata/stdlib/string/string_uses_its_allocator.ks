// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: hello, world 1\nshared 1 forked 2\nlower hello, wörld 2\nslice world\nbuilt abc true\nequal true false\nbalanced true\n

// String and StringBuilder take an allocator like the collections do: every
// buffer a string (and its forks and rebuilt contents) owns comes from it
// and goes back to it when the last copy drops.
module Test

import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator)
import std.text.(StringBuilder)

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
    var s = String[Counting](capacity: 4, allocator: alloc);
    s.append("hello");
    s.append(", world");
    print("\(s) \(live(alloc))\n");

    var t = s.clone();
    let before = live(alloc);
    t.append("!");
    print("shared \(before) forked \(live(alloc))\n");

    var u = String[Counting](allocator: alloc);
    u.append("HELLO, WÖRLD");
    u.lowercase();
    print("lower \(u) \(live(alloc) - 1)\n");

    print("slice \(s.asSlice().subslice(from: 7, to: 12).toOwned())\n");

    var b = StringBuilder[Counting](allocator: alloc);
    b.append("ab");
    b.append(char: 'c');
    let built = b.build();
    print("built \(built) \(built.allocator.live == alloc.live)\n");

    let v = s.clone();
    print("equal \(v == s) \(v == t)\n");
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
