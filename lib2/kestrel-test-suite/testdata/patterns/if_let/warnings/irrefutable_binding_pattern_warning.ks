// test: diagnostics
// stdlib: false

module Main

func test(x: lang.i64) -> lang.i64 {
    if let y = x { // WARNING: irrefutable
        y
    } else {
        0
    }
}
