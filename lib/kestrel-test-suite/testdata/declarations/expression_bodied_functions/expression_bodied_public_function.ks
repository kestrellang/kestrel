// test: execution
// expect-stdout: 99\n

module Main
import std.io.stdio.println

public func publicAnswer() -> std.numeric.Int64 = 99

@main
func main() -> lang.i64 {
    let _ = println(publicAnswer());
    0
}
