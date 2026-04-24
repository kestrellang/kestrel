// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        0 => "zero",
        1 => "one",
        _ => "other"
    }
}
