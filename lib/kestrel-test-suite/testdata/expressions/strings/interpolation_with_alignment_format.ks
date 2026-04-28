// test: diagnostics
// stdlib: true

module Main
import std.io.stdio.println

func main() -> std.numeric.Int64 {
    let val = 42;
    let name = "test";
    let _ = println("[\(val:>8)] [\(name:<10)] [\(val:^6)]");
    0
}
