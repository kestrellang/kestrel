// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `+=` goes through the same trapping `add` as `+`.
@main
func main() -> lang.i64 {
    var x: UInt16 = 65535;
    x += 1;
    if x == 0 { return 1 }
    0
}
