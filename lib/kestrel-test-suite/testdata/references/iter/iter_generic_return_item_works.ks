// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a generic body may RETURN abstract `I.Item` (wrapped in the
// Optional carrier) from a heap-rooted iterator — the accepted side of
// the G3 narrowing. The caller consumes the ref payload through the
// usual pattern binding.
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

func firstItem[I](mutating it: I) -> Optional[I.Item] where I: Iterator {
    it.next()
}

@main
func main() -> lang.i64 {
    let mem = SystemAllocator().allocate(Layout.array[Int64](2)).unwrap().cast[Int64]();
    mem.write(10);
    mem.offset(by: 1).write(20);

    var it = RefRange(base: mem, idx: 0, count: 2);
    if let .Some(w) = firstItem(it) {
        if w != 10 { return 1; }
        // The payload is a live view into the heap buffer.
        mem.write(77);
        if w != 77 { return 2; }
    } else {
        return 3;
    }
    0
}
