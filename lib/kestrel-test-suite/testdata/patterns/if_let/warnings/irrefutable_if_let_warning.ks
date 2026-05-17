// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i64, lang.i64)) -> lang.i64 {
    if let (a, b) = t { // WARNING: irrefutable
        lang.i64_add(a, b)
    } else {
        0
    }
}
