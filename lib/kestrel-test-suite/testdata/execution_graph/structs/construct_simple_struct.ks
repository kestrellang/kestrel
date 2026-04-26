// test: diagnostics
// stdlib: false

module Main

struct Point {
    let x: lang.i64
    let y: lang.i64
}

func makePoint() -> Point {
    Point(x: 3, y: 4)
}
