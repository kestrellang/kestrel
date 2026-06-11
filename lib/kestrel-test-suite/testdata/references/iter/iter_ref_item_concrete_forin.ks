// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: a concrete ref-Item iterator drives for-in end-to-end. The
// desugared `iter()`/`next()` are ProtocolCalls (Callee::Witness even on
// concrete receivers), so this pins witness binding, mono witness
// resolution at `Item = &Int64`, the Optional[&T] carrier return, and
// the `.Some(x)` pattern-payload ref binding in the loop head. The loop
// variable is a true VIEW: re-reading after a write through the same
// storage sees the new value (may-alias).
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

@main
func main() -> lang.i64 {
    let mem = SystemAllocator().allocate(Layout.array[Int64](3)).unwrap().cast[Int64]();
    mem.write(10);
    mem.offset(by: 1).write(20);
    mem.offset(by: 2).write(30);

    var sum = 0;
    for x in RefRange(base: mem, idx: 0, count: 3) {
        sum = sum + x;
    }
    if sum != 60 { return 1; }

    // The ref is a live view, not a snapshot: writing through the
    // underlying storage between reads of `x` is visible.
    for x in RefRange(base: mem, idx: 0, count: 1) {
        let before = x + 0;
        mem.write(99);
        if before != 10 { return 2; }
        if x != 99 { return 3; }
    }
    0
}
