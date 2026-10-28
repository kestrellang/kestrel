// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) facet a2 — a fixed-length pattern `[x, y]` must (1) match ONLY
// length-2 arrays and (2) bind each element by value via `matchGet(0)`/
// `matchGet(1)`. Currently this OSSA-ICEs at build (element access falls
// through to tuple-extract on the array's `{ptr,len,cap}` repr). Pins both the
// exact-length switch (`matchLength() == 2`) and per-index element binding.

module Test

func sum2(arr: [Int64]) -> Int64 {
    match arr {
        [x, y] => x + y,
        _ => -1
    }
}

@main
func main() -> lang.i64 {
    if sum2([5, 8]) != 13 { return 1 }   // both elements bound + read
    if sum2([1, 2, 3]) != -1 { return 2 } // length 3 must NOT match [x, y]
    if sum2([]) != -1 { return 3 }        // length 0 must NOT match [x, y]
    0
}
