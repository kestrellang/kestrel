// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    func magnitude() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}
func caller() -> lang.i64 {
    let pt = Point(x: 1, y: 2);
    pt.magnitude()
}
