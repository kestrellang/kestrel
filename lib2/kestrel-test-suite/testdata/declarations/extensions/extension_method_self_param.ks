// test: diagnostics
// stdlib: false
module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func add(other: Self) -> Self { return Point(x: lang.i64_add(self.x, other.x), y: lang.i64_add(self.y, other.y)); }
}
func test() -> Point {
    let p1 = Point(x: 1, y: 2);
    let p2 = Point(x: 3, y: 4);
    return p1.add(p2);
}
