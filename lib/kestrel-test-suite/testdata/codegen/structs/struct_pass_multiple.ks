// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

func add_points(a: Point, b: Point) -> Point {
    Point(x: a.x + b.x, y: a.y + b.y)
}

@main
func main() -> lang.i64 {
    let p1 = Point(x: 10, y: 5);
    let p2 = Point(x: 12, y: 15);
    let result = add_points(p1, p2);
    // x = 22, y = 20, sum = 42
    if result.x + result.y != 42 { return 1 }
    0
}
