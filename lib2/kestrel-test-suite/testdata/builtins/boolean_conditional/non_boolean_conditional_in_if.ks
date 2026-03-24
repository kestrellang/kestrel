// test: diagnostics
// stdlib: false

module Test
struct NotConditional {
    var value: lang.i64
}
func test(n: NotConditional) -> lang.i64 {
    if n { // ERROR: condition
        1
    } else {
        0
    }
}
