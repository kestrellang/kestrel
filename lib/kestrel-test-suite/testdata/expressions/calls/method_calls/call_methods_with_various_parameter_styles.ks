// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
    func origin() -> lang.i1 { false }
}

struct Calculator {
    let base: lang.i64
    func add(x: lang.i64, y: lang.i64) -> lang.i64 { 42 }
}

struct Formatter {
    let prefix: lang.str
    func format(with value: lang.i64) -> lang.str { "formatted" }
}

func test(p: Point, c: Calculator, f: Formatter) -> lang.i64 {
    c.add(1, 2)
}
