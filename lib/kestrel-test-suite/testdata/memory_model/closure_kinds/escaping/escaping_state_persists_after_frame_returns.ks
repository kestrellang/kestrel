// test: execution
// stdlib: true
// expect-exit: 0

// docs/design/closures.md — escaping state persists across calls once the
// defining frame is gone, and nested closures compose: the inner frame-view
// literal is rooted in the CALL's frame (created and called there), while the
// outer escaping environment keeps `total` alive between calls.
module Test

import std.numeric.Int64

func makeAccumulator(start: Int64) -> escaping (Int64) -> Int64 {
    var total = start;
    { (n) in
        let double = { n * 2 };      // nested normal closure: view of this call's frame
        total = total + double();
        total
    }
}

@main
func main() -> lang.i64 {
    let acc = makeAccumulator(10);
    if acc(3) != 16 { return 1 }   // 10 + 6
    if acc(5) != 26 { return 2 }   // state survived the first call
    0
}
