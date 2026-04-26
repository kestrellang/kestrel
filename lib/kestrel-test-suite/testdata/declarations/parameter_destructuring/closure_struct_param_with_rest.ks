// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> lang.i64 {
    let get_x = { (Point { x, .. }: Point) in x };
    get_x(Point(x: 42, y: 100))
}
