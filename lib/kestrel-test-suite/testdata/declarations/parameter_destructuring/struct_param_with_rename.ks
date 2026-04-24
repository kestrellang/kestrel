// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func sum(Point { x: a, y: b }: Point) -> lang.i64 {
    lang.i64_add(a, b)
}

func test() -> lang.i64 {
    sum(Point(x: 1, y: 2))
}
