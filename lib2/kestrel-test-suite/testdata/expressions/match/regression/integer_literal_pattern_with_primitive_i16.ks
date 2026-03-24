// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i16) -> lang.i64 {
    match x {
        42 => 1,
        _ => 0
    }
}
