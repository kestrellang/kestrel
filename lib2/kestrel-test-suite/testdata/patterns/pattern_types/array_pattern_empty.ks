// test: diagnostics
// stdlib: true

module Main

func test(arr: [lang.i64]) -> lang.str {
    match arr {
        [] => "empty",
        _ => "not empty"
    }
}
