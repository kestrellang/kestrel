// test: diagnostics
// stdlib: false

module Main

func move(from start: lang.i64, to end: lang.i64) -> lang.i64 { end }

func test() -> lang.i64 {
    move(from: 0, to: 10)
}
