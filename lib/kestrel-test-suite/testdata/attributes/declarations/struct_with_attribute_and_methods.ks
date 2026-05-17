// test: diagnostics
// stdlib: false

module Test
@dummy
struct Point {
    var x: lang.i64
    var y: lang.i64

    func magnitude() -> lang.i64 {
        lang.i64_add(self.x, self.y)
    }
}
