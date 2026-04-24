// test: diagnostics
// stdlib: false

module Main

func mixed(x: lang.i64, (a, b): (lang.i64, lang.i64)) -> lang.i64 {
    lang.i64_add(x, lang.i64_add(a, b))
}

func test() -> lang.i64 {
    mixed(10, (1, 2))
}
