// test: diagnostics
// stdlib: true

module Main

struct Point {
    var x: Int
    var y: Int
}

func test() {
    let p = Point(x: 0, y: 0);
    p.x += 10; // ERROR:
}
