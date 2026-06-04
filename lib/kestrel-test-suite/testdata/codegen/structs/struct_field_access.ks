// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

@main
func main() -> lang.i64 {
    let p = Point(x: 10, y: 32);
    if p.x + p.y != 42 { return 1 }
    0
}
