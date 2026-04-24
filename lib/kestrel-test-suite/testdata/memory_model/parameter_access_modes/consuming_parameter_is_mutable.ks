// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func transform(consuming p: Point) -> Point {
    p.x = lang.i64_mul(p.x, 2);
    p.y = lang.i64_mul(p.y, 2);
    p
}
