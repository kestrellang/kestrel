// test: diagnostics
// stdlib: false

module Main

func test(b: lang.i1) -> lang.i64 {
    match b { // ERROR: exhaustive
        true => 1
    }
}
