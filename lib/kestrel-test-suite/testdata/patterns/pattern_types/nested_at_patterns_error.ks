// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        a @ b @ _ => a, // ERROR: @
        _ => 0
    }
}
