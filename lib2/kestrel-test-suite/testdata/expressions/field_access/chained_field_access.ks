// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

struct Line {
    let start: Point
    let end: Point
}

func getStartX(line: Line) -> lang.i64 {
    line.start.x
}
