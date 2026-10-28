// test: diagnostics
// stdlib: true

// Stage 2d: a SHARED ref Item is read-only — assigning through the loop
// variable rejects (same rule as any shared `&` binding).
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
func main() {
    let mem = SystemAllocator().allocate(Layout.array[Int64](1)).unwrap().cast[Int64]();
    mem.write(1);
    for x in RefRange(base: mem, idx: 0, count: 1) {
        x = 5; // ERROR(E208)
    }
}
