// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func tryModify(p: Point) {
    p.x = 10 // ERROR: cannot assign to immutable
}
