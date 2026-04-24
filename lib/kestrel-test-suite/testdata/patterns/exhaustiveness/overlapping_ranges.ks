// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.str {
    match x {
        0..=10 => "first",
        5..=15 => "second", // WARN: overlap
        _ => "other"
    }
}
