// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        10..=0 => "invalid", // ERROR: bound
        _ => "other"
    }
}
