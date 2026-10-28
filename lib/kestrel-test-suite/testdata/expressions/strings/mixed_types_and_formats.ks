// test: diagnostics
// stdlib: true

module Main
import std.io.stdio.println

func main() -> std.numeric.Int64 {
    let name = "Result";
    let value = 42;
    let hex_val = 0xAB;
     println("\(name): \(value:05) (hex: \(hex_val:#x), bin: \(value:b))");
    0
}
