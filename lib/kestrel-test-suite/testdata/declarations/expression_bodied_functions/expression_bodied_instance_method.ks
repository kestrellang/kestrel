// test: execution
// expect-stdout: 7\n

module Main
import std.io.stdio.println

struct Point {
    let x: std.numeric.Int64
    let y: std.numeric.Int64

    func sum() -> std.numeric.Int64 = self.x + self.y
}

@main
func main() -> lang.i64 {
    let p = Point(x: 3, y: 4);
    let _ = println(p.sum());
    0
}
