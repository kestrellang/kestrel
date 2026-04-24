// test: execution
// expect-stdout: 7\n

module Main
import std.io.stdio.println

struct Point {
    let x: std.num.Int64
    let y: std.num.Int64

    func sum() -> std.num.Int64 = self.x + self.y
}

func main() -> std.num.Int64 {
    let p = Point(x: 3, y: 4);
    let _ = println(p.sum());
    0
}
