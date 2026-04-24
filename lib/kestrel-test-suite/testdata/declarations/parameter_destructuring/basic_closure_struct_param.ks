// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> lang.i64 {
    let sum = { (Point { x, y }: Point) in lang.i64_add(x, y) };
    sum(Point(x: 1, y: 2))
}
