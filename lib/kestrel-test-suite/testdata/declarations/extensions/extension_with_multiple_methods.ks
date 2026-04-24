// test: diagnostics
// stdlib: false

module Test

struct Point { var x: lang.i64; var y: lang.i64 }
extend Point {
    func sum() -> lang.i64 { return lang.i64_add(self.x, self.y); }
    func product() -> lang.i64 { return lang.i64_mul(self.x, self.y); }
}
func test() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    return lang.i64_add(p.sum(), p.product());
}
