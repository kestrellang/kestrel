// test: execution
// expect-stdout: 10\n8\n

module Main
import std.io.stdio.println

func max(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 =
    if a > b { a }
    else { b }

@main
func main() -> lang.i64 {
    let _ = println(max(10, 5));
    let _ = println(max(3, 8));
    0
}
