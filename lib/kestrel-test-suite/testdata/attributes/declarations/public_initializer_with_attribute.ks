// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64

    @dummy
    public init(x: lang.i64) {
        self.x = x;
    }
}
