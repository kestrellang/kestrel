// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64
}

func main() -> lang.i64 {
    let p = Point(x: 42, y: 0);
    if p.x != 42 { return 1 }
    0
}
