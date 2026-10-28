// test: diagnostics
// stdlib: false

module Test

func test() -> () -> lang.i64 {
    let x = 42;
    { x } // ERROR(E494)
}
