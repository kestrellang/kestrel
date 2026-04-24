// test: diagnostics
// stdlib: false

module Main

struct Point { var x: lang.i64; var y: lang.i64 }

func test() {
    var p1: Point = Point(x: 0, y: 0);
    let p2: Point = Point(x: 1, y: 1);
    p1 = p2
}
