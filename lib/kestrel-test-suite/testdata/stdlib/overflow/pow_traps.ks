// test: execution
// stdlib: true
// expect-exit: -1

module Test

// Overflow checks are on at -O0 (what tests build with) and off at
// -O1/-O2, where this wraps instead.
// `pow` traps when the result doesn't fit; `powWrapping`,
// `powChecked` and `powSaturating` are the explicit alternatives.
@main
func main() -> lang.i64 {
    let two: Int64 = 2;
    let r = two.pow(63);
    if r == Int64.minValue { return 1 }
    0
}
