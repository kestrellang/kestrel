// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        n => {
            n = 10; // ERROR: immutable
            n
        }
    }
}
