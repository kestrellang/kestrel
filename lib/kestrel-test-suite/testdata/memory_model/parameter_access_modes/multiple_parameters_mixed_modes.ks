// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func process(a: Point, mutating b: Point, consuming c: Point) -> lang.i64 {
    b.x = a.x;
    c.x
}
