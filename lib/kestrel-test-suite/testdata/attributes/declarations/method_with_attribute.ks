// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64

    @dummy
    func getX() -> lang.i64 {
        self.x
    }
}
