// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func reset(mutating point p: Point) {
    p.x = 0;
}
