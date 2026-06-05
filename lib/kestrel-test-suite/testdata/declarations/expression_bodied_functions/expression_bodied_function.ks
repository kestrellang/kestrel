// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

func answer() -> std.numeric.Int64 = 42

@main
func main() -> lang.i64 {
     println(answer());
    0
}
