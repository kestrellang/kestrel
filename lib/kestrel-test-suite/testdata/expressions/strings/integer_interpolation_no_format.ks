// test: diagnostics
// stdlib: true

module Main
import std.io.stdio.println

func main() -> std.numeric.Int64 {
    let a = 42;
    let b = 100;
    let sum = a + b;
    let _ = println("a=\(a), b=\(b), sum=\(sum)");
    0
}
