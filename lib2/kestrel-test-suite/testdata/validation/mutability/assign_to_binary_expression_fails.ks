// test: diagnostics
// stdlib: false

module Test
func test() -> lang.i64 {
    (5 + 10) = 20; // ERROR: cannot assign to this expression
    0
}
