// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

func sum_point(p: Point) -> std.numeric.Int64 {
    p.x + p.y
}

@main
func main() -> lang.i64 {
    let p = Point(x: 20, y: 22);
    if sum_point(p) != 42 { return 1 }
    0
}
