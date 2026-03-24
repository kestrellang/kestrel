// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i8) -> lang.i64 {
    match x {
        0 => 1,
        1 => 2,
        _ => 3
    }
}
