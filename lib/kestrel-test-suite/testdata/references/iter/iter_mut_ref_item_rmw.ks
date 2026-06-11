// test: execution
// backends: cranelift,llvm
// stdlib: true

// Stage 2d: `Item = &mutating T` iteration mutates IN PLACE. The loop
// variable is a pattern-bound `&mutating` local: `x += 1` routes the
// compound assign through the ref (RMW write-through) and `x = v` is a
// store-through — both land in the underlying heap storage.
module Test

struct MutRefRange {
    var base: Pointer[Int64]
    var idx: Int64
    var count: Int64
}

extend MutRefRange: Iterator {
    type Item = &mutating Int64

    mutating func next() -> Optional[&mutating Int64] {
        if self.idx >= self.count { return .None; }
        let r = &mutating self.base.offset(by: self.idx).mutatingValue;
        self.idx = self.idx + 1;
        let o: Optional[&mutating Int64] = .Some(r);
        o
    }
}

@main
func main() -> lang.i64 {
    let mem = SystemAllocator().allocate(Layout.array[Int64](3)).unwrap().cast[Int64]();
    mem.write(10);
    mem.offset(by: 1).write(20);
    mem.offset(by: 2).write(30);

    for x in MutRefRange(base: mem, idx: 0, count: 3) {
        x += 1;
    }
    if mem.read() != 11 { return 1; }
    if mem.offset(by: 1).read() != 21 { return 2; }
    if mem.offset(by: 2).read() != 31 { return 3; }

    for x in MutRefRange(base: mem, idx: 0, count: 3) {
        x = 7;
    }
    if mem.read() != 7 { return 4; }
    if mem.offset(by: 1).read() != 7 { return 5; }
    if mem.offset(by: 2).read() != 7 { return 6; }
    0
}
