// test: execution
// stdlib: true
// expect-exit: 0

// Inside a mutating method: write one of `self`'s array elements, then
// read another field's element on the right of a short-circuit `and`, then
// write that element in the branch. Found in examples/aerie
// (FrameProfile.mark), where the compiler stopped with an internal error:
// "OSSA verify failed ... cannot read ValueId(0): active mut borrow(s)".
// Dropping either the first write or the `and` makes it compile.

module Test

struct Peaks {
    var frames: Int64
    var peaks: [Int64]
    var totals: [Int64]

    mutating func record(i: Int64, took: Int64) {
        self.totals(i) = self.totals(i) + took;
        if self.frames > 0 and took > self.peaks(i) { self.peaks(i) = took; }
    }
}

@main
func main() -> lang.i64 {
    var p = Peaks(frames: 1, peaks: [0, 0], totals: [0, 0]);
    p.record(1, 5);
    p.record(1, 3);
    if p.peaks(1) != 5 { return 1 }
    if p.totals(1) != 8 { return 2 }
    0
}
