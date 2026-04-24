// test: diagnostics
// stdlib: false

module Main

struct Point { var x: lang.i64; var y: lang.i64 }
struct Size { var width: lang.i64; var height: lang.i64 }

func makePoint() -> Point {
    Size(width: 10, height: 20) // ERROR: type mismatch
}
