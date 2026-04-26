// test: diagnostics
// stdlib: false

module Main

func double(x: lang.i64) -> lang.i64 { 42 }
func add(x: lang.i64, y: lang.i64) -> lang.i64 { 42 }

func test() -> lang.i64 {
    add(double(1), double(2))
}
