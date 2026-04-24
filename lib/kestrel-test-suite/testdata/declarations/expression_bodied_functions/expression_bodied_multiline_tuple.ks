// test: execution
// expect-stdout: 3\n7\n

module Main
import std.io.stdio.println

func makePair(a: std.num.Int64, b: std.num.Int64) -> (std.num.Int64, std.num.Int64) =
    (
        a,
        b
    )

func main() -> std.num.Int64 {
    let (x, y) = makePair(3, 7);
    let _ = println(x);
    let _ = println(y);
    0
}
