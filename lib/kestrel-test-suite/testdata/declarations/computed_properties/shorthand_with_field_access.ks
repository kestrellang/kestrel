// test: diagnostics
// stdlib: false

module Test

struct Point {
    var x: lang.i64
    var y: lang.i64
}
struct Line {
    var start: Point
    var end: Point

    var length: lang.i64 {
        lang.i64_sub(self.end.x, self.start.x)
    }
}
