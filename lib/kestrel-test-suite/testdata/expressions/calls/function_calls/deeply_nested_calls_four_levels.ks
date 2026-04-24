// test: diagnostics
// stdlib: false

module Main

func id(x: lang.i64) -> lang.i64 { x }

func test() -> lang.i64 {
    id(id(id(id(42))))
}
