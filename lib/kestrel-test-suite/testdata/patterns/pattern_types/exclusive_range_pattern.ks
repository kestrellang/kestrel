// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        0..<10 => "single digit",
        _ => "other"
    }
}
