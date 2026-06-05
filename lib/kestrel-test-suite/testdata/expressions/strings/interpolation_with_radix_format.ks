// test: diagnostics
// stdlib: true

module Main
import std.io.stdio.println

func main() -> std.numeric.Int64 {
    let val = 255;
     println("hex:\(val:x) HEX:\(val:#X) bin:\(val:#b) pad:\(val:08x)");
    0
}
