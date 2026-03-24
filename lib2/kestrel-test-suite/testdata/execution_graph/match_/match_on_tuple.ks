// test: diagnostics
// stdlib: false

module Main

func classify(pair: (lang.i64, lang.i64)) -> lang.i64 {
    match pair {
        (0, 0) => 0,
        (0, _) => 1,
        (_, 0) => 2,
        _ => 3
    }
}
