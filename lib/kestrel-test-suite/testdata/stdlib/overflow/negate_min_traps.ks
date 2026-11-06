// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// Negating minValue has no representable result, so it traps.
@main
func main() -> lang.i64 {
    let m = Int8.minValue;
    let n = -m;
    if n == Int8.minValue { return 1 }
    0
}
