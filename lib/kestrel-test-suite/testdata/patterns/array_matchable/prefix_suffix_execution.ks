// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) facet a3 (suffix side) — `[first, .., last]` must match arrays
// of length >= 2, bind `first` via `matchGet(0)` and `last` via
// `matchGet(matchLength() - 1)` (the IndexFromEnd path, currently a no-op in
// MIR so `last` reads garbage). Pins the minimum-length switch plus end-relative
// element access.

module Test

func ends(arr: [Int64]) -> Int64 {
    match arr {
        [first, .., last] => first * 100 + last,
        [only] => only,
        [] => -1
    }
}

@main
func main() -> lang.i64 {
    if ends([3, 7, 9]) != 309 { return 1 }   // first=3 (matchGet 0), last=9 (from end)
    if ends([4, 5]) != 405 { return 2 }      // exactly 2: first=4, last=5
    if ends([8]) != 8 { return 3 }           // length 1 falls to `[only]`
    if ends([]) != -1 { return 4 }           // length 0 falls to `[]`
    0
}
