// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

struct Line {
    var start: Point
    var end: Point
}

func test() -> lang.i64 {
    let line = Line(start: Point(x: 0, y: 0), end: Point(x: 10, y: 10));
    let Line { start: Point { x: x1, .. }, end: Point { x: x2, .. } } = line;
    lang.i64_sub(x2, x1)
}
