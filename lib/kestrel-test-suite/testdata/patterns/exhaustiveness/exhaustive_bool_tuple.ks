// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i1, lang.i1)) -> lang.i64 {
    match t {
        (true, true) => 1,
        (true, false) => 2,
        (false, true) => 3,
        (false, false) => 4
    }
}
