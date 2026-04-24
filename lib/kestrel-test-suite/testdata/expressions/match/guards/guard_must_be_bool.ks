// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        n if n => n, // ERROR: Bool
        _ => 0
    }
}
