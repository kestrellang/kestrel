// test: execution
// stdlib: true

module Test

struct Point {
    var x: std.numeric.Int64
    var y: std.numeric.Int64
}

func make_and_modify(base: std.numeric.Int64) -> Point {
    var p = Point(x: base, y: base);
    p.x = p.x + 10;
    p.y = p.y + 12;
    p
}

func main() -> lang.i64 {
    let p = make_and_modify(10);
    // x = 10 + 10 = 20, y = 10 + 12 = 22, sum = 42
    if p.x + p.y != 42 { return 1 }
    0
}
