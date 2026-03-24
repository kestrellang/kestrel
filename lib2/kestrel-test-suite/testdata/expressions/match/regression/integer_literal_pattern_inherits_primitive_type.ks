// test: diagnostics
// stdlib: false

module Main

func classify(code: lang.i32) -> lang.i64 {
    match code {
        0 => 1,
        1 => 2,
        2 => 3,
        _ => 0
    }
}
