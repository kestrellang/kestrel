// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}

extend Point {
    func magnitude() -> lang.i64 {
        lang.i64_add(
            lang.i64_mul(self.x, self.x),
            lang.i64_mul(self.y, self.y)
        )
    }
}

func test() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    p.magnitude()
}
