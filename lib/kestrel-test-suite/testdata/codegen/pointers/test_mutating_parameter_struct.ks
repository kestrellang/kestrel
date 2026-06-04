// test: execution
// stdlib: true

module Test

struct Point {
    var x: std.numeric.Int64
    var y: std.numeric.Int64
}

func reset(mutating p: Point) {
    p.x = 42;
    p.y = 0;
}

@main
func main() -> lang.i64 {
    var pt = Point(x: 0, y: 0);
    reset(pt);
    if pt.x != 42 { return 1 }
    0
}
