// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        0..=9 => "digit",
        _ => "other"
    }
}
