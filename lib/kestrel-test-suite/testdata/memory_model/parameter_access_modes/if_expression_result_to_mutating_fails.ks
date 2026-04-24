// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func reset(mutating p: Point) {
    p.x = 0;
}
func test(cond: lang.i1) {
    reset(if cond { Point(x: 1, y: 2) } else { Point(x: 3, y: 4) }) // ERROR: mutating
}
