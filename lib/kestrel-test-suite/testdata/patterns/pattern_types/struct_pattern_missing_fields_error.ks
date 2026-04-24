// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test(p: Point) -> lang.i64 {
    match p {
        Point { x } => x // ERROR: y
    }
}
