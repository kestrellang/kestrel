// test: diagnostics
// stdlib: false

module Test
struct Point {
    var x: lang.i64

    @dummy
    static func origin() -> Point {
        Point(x: 0)
    }
}
