// test: diagnostics
// stdlib: false
module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func clone() -> Self { return Point(x: self.x, y: self.y); }
}
func test() -> Point {
    let p = Point(x: 1, y: 2);
    return p.clone();
}
