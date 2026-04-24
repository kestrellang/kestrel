// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func get_x(Point { x, .. }: Point) -> lang.i64 {
    x
}

func test() -> lang.i64 {
    get_x(Point(x: 42, y: 100))
}
