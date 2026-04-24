// test: diagnostics
// stdlib: false
// skip: Array literal patterns not yet supported

module Main

func test(arr: [lang.i64]) -> lang.str {
    match arr {
        [1, 2, 3] => "one two three",
        [0, ..] => "starts with zero",
        _ => "other"
    }
}
