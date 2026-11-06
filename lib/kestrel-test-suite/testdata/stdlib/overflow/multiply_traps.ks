// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `*` traps when the product doesn't fit.
@main
func main() -> lang.i64 {
    let big: Int32 = 100000;
    let product = big * big;
    if product == 1410065408 { return 1 }
    0
}
