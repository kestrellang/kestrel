// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: GENERIC dispatch over a ref-Item iterator. The where-clause
// equality RHS may itself be a ref (`I.Item = &Int64` — the 2d carve;
// nested non-aggregate refs and protocol-bound args stay strict), the
// witness calls resolve at mono with the ref Item, and the
// `extend Iterator: Iterable` blanket carries generic for-in.
module Test

struct RefRange {
    var base: Pointer[Int64]
    var idx: Int64
    var count: Int64
}

extend RefRange: Iterator {
    type Item = &Int64

    mutating func next() -> Optional[&Int64] {
        if self.idx >= self.count { return .None; }
        let r = &self.base.offset(by: self.idx).value;
        self.idx = self.idx + 1;
        let o: Optional[&Int64] = .Some(r);
        o
    }
}

func drain[I](mutating it: I) -> Int64 where I: Iterator, I.Item = &Int64 {
    var sum = 0;
    while let .Some(x) = it.next() {
        sum = sum + x;
    }
    sum
}

func drainForIn[I](it: I) -> Int64 where I: Iterator, I.Item = &Int64 {
    var sum = 0;
    for x in it {
        sum = sum + x;
    }
    sum
}

@main
func main() -> lang.i64 {
    let mem = SystemAllocator().allocate(Layout.array[Int64](3)).unwrap().cast[Int64]();
    mem.write(10);
    mem.offset(by: 1).write(20);
    mem.offset(by: 2).write(30);

    var a = RefRange(base: mem, idx: 0, count: 3);
    if drain(a) != 60 { return 1; }

    let b = RefRange(base: mem, idx: 0, count: 3);
    if drainForIn(b) != 60 { return 2; }
    0
}
