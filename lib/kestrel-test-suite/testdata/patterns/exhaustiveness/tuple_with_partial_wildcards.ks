// test: diagnostics
// stdlib: false

module Main

func test(t: (lang.i1, lang.i1)) -> lang.i64 {
    match t {
        (true, _) => 1,
        (false, _) => 0
    }
}
