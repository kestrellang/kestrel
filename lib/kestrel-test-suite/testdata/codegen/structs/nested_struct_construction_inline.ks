// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

struct Rectangle {
    let origin: Point
    let size: Point
}

@main
func main() -> lang.i64 {
    let rect = Rectangle(
        origin: Point(x: 5, y: 10),
        size: Point(x: 20, y: 7)
    );
    // 5 + 10 + 20 + 7 = 42
    if rect.origin.x + rect.origin.y + rect.size.x + rect.size.y != 42 { return 1 }
    0
}
