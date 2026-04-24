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

func test(line: Line) -> lang.i64 {
    match line {
        Line { start: Point { x: x1, y: y1 }, end: Point { x: x2, y: y2 } } => lang.i64_add(lang.i64_add(lang.i64_add(x1, y1), x2), y2)
    }
}
