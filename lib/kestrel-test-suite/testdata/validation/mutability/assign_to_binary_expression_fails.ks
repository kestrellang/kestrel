// test: diagnostics
// stdlib: true

module Test
func test() -> Int64 {
    (5 + 10) = 20; // ERROR: cannot assign to this expression
    0
}
