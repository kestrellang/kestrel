// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> Point {
    Point(a: 1, b: 2) // ERROR: label
}
