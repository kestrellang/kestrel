// test: execution
// stdlib: true
// expect-exit: 0
//
// #188 (BUG-52) — the full `[a, ..rest, z]` shape: match length >= 2, bind the
// first element (`matchGet(0)`), the last (`matchGet(matchLength()-1)`), and
// the middle slice (`matchSlice(1, matchLength()-1)`). Exercises prefix +
// rest + suffix in one pattern, on both backends.

module Test

func describe(arr: [Int64]) -> Int64 {
    match arr {
        [a, ..rest, z] => a * 1000 + rest.count * 10 + z,
        _ => -1
    }
}

@main
func main() -> lang.i32 {
    if describe([1, 2, 3, 4]) != 1024 { return 1 } // a=1, rest=[2,3] (2), z=4
    if describe([5, 9]) != 5009 { return 2 }       // a=5, rest=[] (0),    z=9
    if describe([7]) != -1 { return 3 }            // length 1 < min 2
    if describe([]) != -1 { return 4 }
    0
}
