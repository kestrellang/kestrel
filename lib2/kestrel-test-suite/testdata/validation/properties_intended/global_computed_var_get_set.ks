// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

module Main
import std.io.stdio.println

private var _g: std.num.Int64 = 1;

public var globalComputedVar: std.num.Int64 {
    get { _g }
    set { _g = newValue }
}

func main() -> std.num.Int64 {
    let _ = println(globalComputedVar);
    globalComputedVar = 2;
    let _ = println(globalComputedVar);
    0
}
