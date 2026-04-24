// test: diagnostics
// stdlib: false

module Main

func double(x: lang.i64) -> lang.i64 { 42 }

func test() -> lang.i64 {
    double(1, 2) // ERROR: no matching overload
}
