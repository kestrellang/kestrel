// test: diagnostics
// stdlib: false

module Main

func describe(n: lang.i64) -> lang.i64 {
    match n {
        0 => 0,
        1 => 1,
        _ => 2
    }
}
