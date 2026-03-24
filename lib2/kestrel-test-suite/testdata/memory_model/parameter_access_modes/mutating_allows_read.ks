// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func double(mutating p: Point) {
    p.x = lang.i64_mul(p.x, 2);
}
