// test: diagnostics
// stdlib: false

module Test
func test() -> lang.i64 {
    5 = 10; // ERROR: cannot assign to this expression
    0
}
