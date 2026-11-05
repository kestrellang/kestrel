// test: execution
// stdlib: true
// expect-stdout: 7\n

module Main
import std.streams.println

public let globalLet: std.numeric.Int64 = 7;

@main
func main() -> lang.i64 {
     println(globalLet);
    0
}
