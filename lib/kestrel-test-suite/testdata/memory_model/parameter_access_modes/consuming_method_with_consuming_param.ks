// test: diagnostics
// stdlib: false

module Test
struct Point { var x: lang.i64; var y: lang.i64 }
struct Container {
    var point: Point

    consuming func takePoint() -> lang.i64 {
        consume(self.point)
    }
}
func consume(consuming p: Point) -> lang.i64 {
    p.x
}
