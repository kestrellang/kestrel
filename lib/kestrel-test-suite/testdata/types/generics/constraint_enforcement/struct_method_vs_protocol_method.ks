// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
    func getX() -> lang.i64 {
        return x
    }
}
func usePoint(p: Point) -> lang.i64 {
    return p.getX()
}
