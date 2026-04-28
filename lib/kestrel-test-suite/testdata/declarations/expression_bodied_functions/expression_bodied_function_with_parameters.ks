// test: execution
// expect-stdout: 7\n

module Main
import std.io.stdio.println

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 = a + b

func main() -> std.numeric.Int64 {
    let _ = println(add(3, 4));
    0
}
