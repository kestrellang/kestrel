// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64
    var y: lang.i64

    func copyXTo(mutating other: Point) {
        other.x = self.x;
    }
}
func caller() {
    let pt1 = Point(x: 1, y: 2);
    var pt2 = Point(x: 3, y: 4);
    pt1.copyXTo(pt2)
}
