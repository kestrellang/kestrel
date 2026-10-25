// test: diagnostics
// stdlib: false

module Main

func test(cond: lang.i1) -> lang.i64 {
    if cond {
        true // ERROR: type 'i64' does not accept bool literal
    } else {
        42
    }
}
