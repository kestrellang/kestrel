// test: diagnostics
// stdlib: false

module Test
func getValue() -> lang.i64 { 5 }
func test() -> lang.i64 {
    getValue() = 10; // ERROR: cannot assign to this expression
    0
}
