// test: execution
// expect-stdout: 42\n

module Main
import std.io.stdio.println

func identity[T](x: T) -> T = x

func main() -> std.num.Int64 {
    let _ = println(identity(42));
    0
}
