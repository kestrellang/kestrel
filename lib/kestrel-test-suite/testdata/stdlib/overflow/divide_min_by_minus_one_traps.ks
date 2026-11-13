// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `minValue / -1` overflows, so it traps like the other operators.
@main
func main() -> lang.i64 {
    let m = Int64.minValue;
    let q = m / -1;
    if q == Int64.minValue { return 1 }
    0
}
