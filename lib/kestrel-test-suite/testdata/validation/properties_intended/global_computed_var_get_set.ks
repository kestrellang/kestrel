// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

module Main
import std.io.stdio.println

private var _g: std.numeric.Int64 = 1;

public var globalComputedVar: std.numeric.Int64 {
    get { _g }
    set { _g = newValue }
}

@main
func main() -> lang.i64 {
    let _ = println(globalComputedVar);
    globalComputedVar = 2;
    let _ = println(globalComputedVar);
    0
}
