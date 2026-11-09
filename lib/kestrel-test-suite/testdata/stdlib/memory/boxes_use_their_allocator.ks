// test: execution
// stdlib: true
// expect-exit: 0
// expect-stdout: owned 42 2\nrc 7 1 2\ncow 1 2 2\nunique 9 0\nsizes 8 16 8 16\nindirect 6\nclosure 3\nbalanced true\n

// OwnedBox, RcBox, CowBox and UniqueBox store their allocator: every block
// they (and their clones and forks) allocate comes from it and goes back to
// it. A stateless allocator adds no bytes, so the default boxes stay one
// word and still back indirect enums and escaping closures.
module Test

import std.memory.(Allocator, Layout, Pointer, RawPointer, SystemAllocator, OwnedBox, RcBox, CowBox, UniqueBox)

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

indirect enum List {
    case Nil
    case Cons(Int64, List)
}

func sum(l: List) -> Int64 {
    match l {
        .Nil => 0,
        .Cons(x, rest) => x + sum(rest),
    }
}

func adder(n: Int64) -> escaping (Int64) -> Int64 {
    { (x) in x + n }
}

func useAll(alloc: Counting) {
    let o = OwnedBox[Int64, Counting](42, allocator: alloc);
    let o2 = o.clone();
    print("owned \(o2.value) \(live(alloc))\n");

    let r = RcBox[Int64, Counting](7, allocator: alloc);
    let r2 = r.clone();
    let shared = live(alloc) - 2;
    let r3 = r2.deepClone();
    print("rc \(r3.value) \(shared) \(live(alloc) - 2)\n");

    var c = CowBox[String, Counting]("1", allocator: alloc);
    var c2 = c.clone();
    c2.value = "2";
    print("cow \(c.value) \(c2.value) \(live(alloc) - 4)\n");

    let u = UniqueBox[Int64, Counting](9, allocator: alloc);
    let before = live(alloc);
    let nine = u.takeValue();
    u.destroy();
    print("unique \(nine) \(before - live(alloc) - 1)\n");
}

@main
func main() {
    var sys = SystemAllocator();
    let raw = match sys.allocate(Layout.of[Int64]()) { .Some(p) => p, .None => fatalError("oom") };
    let counter = raw.cast[Int64]();
    unsafe counter.write(0);
    let alloc = Counting(live: counter);
    useAll(alloc);
    print("sizes \(Layout.of[OwnedBox[Int64]]().size) \(Layout.of[OwnedBox[Int64, Counting]]().size) \(Layout.of[RcBox[Int64]]().size) \(Layout.of[RcBox[Int64, Counting]]().size)\n");
    print("indirect \(sum(.Cons(1, .Cons(2, .Cons(3, .Nil)))))\n");
    let f = adder(2);
    print("closure \(f(1))\n");
    print("balanced \(live(alloc) == 0)\n");
    unsafe sys.deallocate(raw, Layout.of[Int64]());
}
