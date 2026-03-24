// test: diagnostics
// stdlib: false

module Main

func build(first x: lang.i64, second y: lang.i64, third z: lang.i64) -> lang.i64 {
    42
}

func test() -> lang.i64 {
    build(first: 1, second: 2, third: 3)
}
