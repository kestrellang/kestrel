// test: diagnostics
// stdlib: false

module Test

func test(x: lang.i64) -> lang.i64 {
    x = 10; // ERROR: cannot assign to immutable
    x
}
