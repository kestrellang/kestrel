// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        10..=0 => 1, // ERROR: bound
        _ => 0
    }
}
