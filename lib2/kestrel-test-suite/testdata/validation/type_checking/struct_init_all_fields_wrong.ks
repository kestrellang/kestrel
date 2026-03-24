// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p: Point = Point(x: "a", y: "b"); // ERROR
}
