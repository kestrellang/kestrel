// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64)) -> lang.str {
    match t {
        (0, 0) => "origin",
        (0, _) => "y-axis",
        (_, 0) => "x-axis",
        _ => "elsewhere"
    }
}
