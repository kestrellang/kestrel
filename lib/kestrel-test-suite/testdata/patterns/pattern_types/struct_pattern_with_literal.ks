// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.str {
    match p {
        Point { x: 0, y: 0 } => "origin",
        Point { x: 0, y } => "y-axis",
        Point { x, y: 0 } => "x-axis",
        Point { .. } => "elsewhere"
    }
}
