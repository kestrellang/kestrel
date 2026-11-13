// test: execution
// expect-stdout: 99\n

module Main
import std.streams.println

public func publicAnswer() -> std.numeric.Int64 = 99

@main
func main() -> lang.i64 {
     println(publicAnswer());
    0
}
