// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

func identity[T](x: T) -> T = x

@main
func main() -> lang.i64 {
     println(identity(42));
    0
}
