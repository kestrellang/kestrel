// test: execution
// stdlib: true
// expect-exit: 0
//
// #186 (BUG-50): an open-ended range-from pattern `N..` used to lower to an
// always-false test (the absent upper bound was filled with the sentinel
// i64::MAX, which truncated to -1 under the narrowed switch discriminant), so
// it never matched — and when it was the final arm of an exhaustive match the
// value fell off the end into a trap (SIGILL). The fix leaves an open bound
// untested. Exercises both the with-wildcard and exhaustive (no-wildcard)
// shapes, plus the prefix `..<N` and closed/inclusive forms for good measure.

module Test

func withWildcard(x: Int64) -> Int64 {
    match x {
        10.. => 1,
        _ => 0
    }
}

func exhaustive(x: Int64) -> Int64 {
    match x {
        ..<0 => 1,
        0..=9 => 2,
        10..<20 => 3,
        20.. => 4
    }
}

@main
func main() -> lang.i32 {
    // range-from with a wildcard fallback
    if withWildcard(42) != 1 { return 1 }
    if withWildcard(10) != 1 { return 2 }
    if withWildcard(9) != 0 { return 3 }

    // exhaustive match whose final arm is an open range-from (used to trap)
    if exhaustive(-5) != 1 { return 4 }
    if exhaustive(3) != 2 { return 5 }
    if exhaustive(15) != 3 { return 6 }
    if exhaustive(99) != 4 { return 7 }
    if exhaustive(20) != 4 { return 8 }
    0
}
