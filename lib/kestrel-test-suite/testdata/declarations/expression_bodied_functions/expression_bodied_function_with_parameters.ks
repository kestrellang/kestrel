// test: execution
// expect-stdout: 7\n

module Main
import std.io.stdio.println

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 = a + b

@main
func main() -> lang.i64 {
    let _ = println(add(3, 4));
    0
}
