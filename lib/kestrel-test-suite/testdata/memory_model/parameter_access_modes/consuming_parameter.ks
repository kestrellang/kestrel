// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
func consume(consuming p: Point) -> lang.i64 {
    p.x
}
