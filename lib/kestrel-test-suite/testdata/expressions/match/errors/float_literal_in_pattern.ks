// test: diagnostics
// stdlib: false

module Main

func test(x: lang.f64) -> lang.i64 {
    match x {
        3.14 => 1, // ERROR: float
        _ => 0
    }
}
