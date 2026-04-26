// test: diagnostics
// stdlib: true

module Main
import std.io.stdio.println

func main() -> std.num.Int64 {
    let first = "Hello";
    let second = "World";
    let third = "Kestrel";
    let _ = println("\(first), \(second)! Welcome to \(third).");
    0
}
