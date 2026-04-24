// test: execution
// expect-stdout: 10\n8\n

module Main
import std.io.stdio.println

func max(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 =
    if a > b { a }
    else { b }

func main() -> std.num.Int64 {
    let _ = println(max(10, 5));
    let _ = println(max(3, 8));
    0
}
