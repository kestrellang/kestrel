// test: diagnostics
// stdlib: false
module Test
struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> Point {
    Point(x: 1, y: 2, z: 3) // ERROR: has 2 field(s)
}
