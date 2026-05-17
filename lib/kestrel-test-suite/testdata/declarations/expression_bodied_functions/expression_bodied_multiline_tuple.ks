// test: execution
// expect-stdout: 3\n7\n

module Main
import std.io.stdio.println

func makePair(a: std.numeric.Int64, b: std.numeric.Int64) -> (std.numeric.Int64, std.numeric.Int64) =
    (
        a,
        b
    )

func main() -> std.numeric.Int64 {
    let (x, y) = makePair(3, 7);
    let _ = println(x);
    let _ = println(y);
    0
}
