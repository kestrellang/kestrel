// test: diagnostics
// stdlib: false

module Main

func test(s: lang.str) -> lang.i64 {
    match s {
        "hello" => 1,
        "world" => 2,
        _ => 0
    }
}
