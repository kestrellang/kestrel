// test: diagnostics
// stdlib: false

module Main

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func test() -> lang.i64 {
    let p = Point(x: 1, y: 2);
    let Point { x, y } = p;
    lang.i64_add(x, y)
}
