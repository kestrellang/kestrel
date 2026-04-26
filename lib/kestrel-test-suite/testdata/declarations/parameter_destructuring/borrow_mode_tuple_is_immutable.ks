// test: diagnostics
// stdlib: false

module Main

func read((a, b): (lang.i64, lang.i64)) -> lang.i64 {
    // a and b are immutable in borrow mode
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    read((1, 2))
}
