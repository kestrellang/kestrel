// test: diagnostics
// stdlib: false

module Main

struct Calculator {
    let base: lang.i64
    func getValue() -> lang.i64 { 42 }
    func add(x: lang.i64) -> lang.i64 { 42 }
    func multiply(x: lang.i64, y: lang.i64) -> lang.i64 { 42 }
}

func test(c: Calculator) -> lang.i64 {
    c.multiply(5, 6)
}
