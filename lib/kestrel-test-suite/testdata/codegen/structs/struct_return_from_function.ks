// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

func make_point(x: std.numeric.Int64, y: std.numeric.Int64) -> Point {
    Point(x: x, y: y)
}

@main
func main() -> lang.i64 {
    let p = make_point(10, 32);
    if p.x + p.y != 42 { return 1 }
    0
}
