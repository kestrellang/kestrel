// test: execution
// stdlib: true
// expect-stdout: 10\n

module Main
import std.io.stdio.println

public struct Foo {
    public static let structStaticLet: std.numeric.Int64 = 10;
}

func main() -> std.numeric.Int64 {
    let _ = println(Foo.structStaticLet);
    0
}
