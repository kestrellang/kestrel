// test: diagnostics
// stdlib: false

module Main

func inner() -> lang.i64 { 42 }
func outer(x: lang.i64) -> lang.i64 { x }

func test() -> lang.i64 {
    outer(inner())
}
