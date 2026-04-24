// test: diagnostics
// stdlib: false

module Main

struct Point { var x: lang.i64; var y: lang.i64 }
struct Size { var width: lang.i64; var height: lang.i64 }

func usePoint(p: Point) {}

func test() {
    usePoint(Size(width: 10, height: 20)) // ERROR: type mismatch
}
