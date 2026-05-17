// test: execution
// stdlib: true

module Test

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64

    func sum() -> std.numeric.Int64 {
        self.x + self.y
    }
}

func main() -> lang.i64 {
    let pt = Point(x: 20, y: 22);
    if pt.sum() != 42 { return 1 }
    0
}
