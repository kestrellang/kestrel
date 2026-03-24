// test: diagnostics
// stdlib: false

module Test

func test() -> lang.i64 {
    let x: lang.i64 = 5;
    x = 10; // ERROR: cannot assign to immutable
    x
}
