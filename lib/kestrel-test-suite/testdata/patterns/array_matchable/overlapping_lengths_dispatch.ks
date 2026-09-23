// test: execution
// stdlib: true
// expect-exit: 0
//
// F1 — array patterns match a range of lengths (`[x, ..]` is length >= 1), so
// they overlap like integer ranges do. A value must reach the first arm whose
// length range contains it. `longFirst([7])` used to take `[x, y, ..]` on a
// one-element array, and `exactFirst([7, 8])` used to take `[x]`.

module Test

func longFirst(arr: [Int64]) -> Int64 {
    match arr {
        [x, y, ..] => 2,
        [x, ..] => 1,
        _ => 0
    }
}

func exactFirst(arr: [Int64]) -> Int64 {
    match arr {
        [x] => 1,
        [x, ..] => 2,
        _ => 0
    }
}

// Element tests on a rest pattern checked against an exact length: `[x, .., 3]`
// on a two-element array must test the LAST element against 3.
func suffixTest(arr: [Int64]) -> Int64 {
    match arr {
        [1, 2] => 1,
        [x, .., 3] => 2,
        _ => 0
    }
}

// Two rest patterns with different prefix/suffix lengths.
func twoRests(arr: [Int64]) -> Int64 {
    match arr {
        [1, ..] => 1,
        [.., 9, 9] => 2,
        _ => 0
    }
}

@main
func main() -> lang.i32 {
    if longFirst([7]) != 1 { return 1 }
    if longFirst([7, 8]) != 2 { return 2 }
    if longFirst([]) != 0 { return 3 }
    if exactFirst([7]) != 1 { return 4 }
    if exactFirst([7, 8]) != 2 { return 5 }
    if exactFirst([]) != 0 { return 6 }
    if suffixTest([1, 2]) != 1 { return 7 }
    if suffixTest([5, 3]) != 2 { return 8 }
    if suffixTest([5, 4]) != 0 { return 9 }
    if suffixTest([5, 6, 3]) != 2 { return 10 }
    if suffixTest([5, 6, 4]) != 0 { return 11 }
    if twoRests([1, 9, 9]) != 1 { return 12 }
    if twoRests([2, 9, 9]) != 2 { return 13 }
    if twoRests([2, 8, 9]) != 0 { return 14 }
    if twoRests([9, 9]) != 2 { return 15 }
    0
}
