// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    match x {
        "hello" => 1, // ERROR: type
        _ => 0
    }
}
