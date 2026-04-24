// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() {
    let p = Point(x: 1, y: 2);
    let x: lang.i64 = p.x;
}
