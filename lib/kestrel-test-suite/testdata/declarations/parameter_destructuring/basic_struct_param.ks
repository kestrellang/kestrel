// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func sum(Point { x, y }: Point) -> lang.i64 {
    lang.i64_add(x, y)
}

func test() -> lang.i64 {
    sum(Point(x: 1, y: 2))
}
