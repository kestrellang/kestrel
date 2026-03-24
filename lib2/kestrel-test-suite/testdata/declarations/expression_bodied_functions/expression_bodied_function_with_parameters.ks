// test: execution
// expect-stdout: 7\n

module Main
import std.io.stdio.println

func add(a: std.num.Int64, b: std.num.Int64) -> std.num.Int64 = a + b

func main() -> std.num.Int64 {
    let _ = println(add(3, 4));
    0
}
