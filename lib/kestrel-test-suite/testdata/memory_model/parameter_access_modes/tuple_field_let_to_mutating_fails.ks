// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func reset(mutating p: Point) {
    p.x = 0;
}
func test() {
    let tuple = (Point(x: 1, y: 2), 42);
    reset(tuple.0) // ERROR: mutating
}
