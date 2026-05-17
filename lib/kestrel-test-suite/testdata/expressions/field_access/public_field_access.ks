// test: diagnostics
// stdlib: false

module Main

struct Point {
    public let x: lang.i64
    public let y: lang.i64
}

func getX(p: Point) -> lang.i64 {
    p.x
}

func getY(p: Point) -> lang.i64 {
    p.y
}
