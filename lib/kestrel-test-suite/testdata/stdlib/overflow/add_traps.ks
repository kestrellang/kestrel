// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `+` traps on overflow instead of silently wrapping to minValue.
// Use `addWrapping` when wrapping is intended.
@main
func main() -> lang.i64 {
    let m = Int64.maxValue;
    let wrapped = m + 1;
    if wrapped == Int64.minValue { return 1 }
    0
}
