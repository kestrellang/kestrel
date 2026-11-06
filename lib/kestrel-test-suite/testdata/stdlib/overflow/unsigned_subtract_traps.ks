// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `0 - 1` on an unsigned type traps instead of wrapping to 255.
@main
func main() -> lang.i64 {
    let zero: UInt8 = 0;
    let wrapped = zero - 1;
    if wrapped == 255 { return 1 }
    0
}
