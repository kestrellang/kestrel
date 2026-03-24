// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func getX(p: Point) -> lang.i64 {
    p.x
}
