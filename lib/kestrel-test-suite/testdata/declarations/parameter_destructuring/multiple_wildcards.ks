// test: diagnostics
// stdlib: false

module Main

func middle((_, b, _): (lang.i64, lang.i64, lang.i64)) -> lang.i64 {
    b
}

func test() -> lang.i64 {
    middle((1, 42, 3))
}
