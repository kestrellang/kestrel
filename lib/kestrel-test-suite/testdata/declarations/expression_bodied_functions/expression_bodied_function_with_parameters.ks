// test: execution
// expect-stdout: 7\n

module Main
import std.streams.println

func add(a: std.numeric.Int64, b: std.numeric.Int64) -> std.numeric.Int64 = a + b

@main
func main() -> lang.i64 {
     println(add(3, 4));
    0
}
