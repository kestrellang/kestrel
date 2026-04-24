// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Shape {
    var origin: Point

    func tryReset() {
        reset(self.origin) // ERROR: mutating
    }
}
func reset(mutating p: Point) {
    p.x = 0;
}
