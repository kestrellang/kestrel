// test: diagnostics
// stdlib: false
module Test
struct Point { var x: lang.i64
 var y: lang.i64 }
func makePoint() -> Point { Point(x: 1, y: 2) }
