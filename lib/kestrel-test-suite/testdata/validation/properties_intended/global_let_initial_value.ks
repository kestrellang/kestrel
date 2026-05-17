// test: execution
// stdlib: true
// expect-stdout: 7\n

module Main
import std.io.stdio.println

public let globalLet: std.numeric.Int64 = 7;

func main() -> std.numeric.Int64 {
    let _ = println(globalLet);
    0
}
