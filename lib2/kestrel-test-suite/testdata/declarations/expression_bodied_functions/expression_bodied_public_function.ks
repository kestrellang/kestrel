// test: execution
// expect-stdout: 99\n

module Main
import std.io.stdio.println

public func publicAnswer() -> std.num.Int64 = 99

func main() -> std.num.Int64 {
    let _ = println(publicAnswer());
    0
}
