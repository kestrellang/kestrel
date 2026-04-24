// test: execution
// stdlib: true
// expect-stdout: 0\n5\n

module Main
import std.io.stdio.println

public var globalVar: std.num.Int64 = 0;

func main() -> std.num.Int64 {
    let _ = println(globalVar);
    globalVar = 5;
    let _ = println(globalVar);
    0
}
