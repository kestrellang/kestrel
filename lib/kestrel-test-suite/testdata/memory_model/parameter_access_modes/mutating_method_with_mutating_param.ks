// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Shape {
    var origin: Point

    mutating func resetOrigin() {
        reset(self.origin)
    }
}
func reset(mutating p: Point) {
    p.x = 0;
    p.y = 0;
}
