// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64
    var y: lang.i64
}
func getX(p: Point) -> lang.i64 {
    match p {
        Point { x: xVal, y: _ } => xVal
    }
}
