// test: execution
// stdlib: true
// expect-stdout: 11\n

module Main
import std.io.stdio.println

public struct Foo {
    public let structLet: std.numeric.Int64 = 11;
}

func main() -> std.numeric.Int64 {
    let foo = Foo(structLet: 11);
    let _ = println(foo.structLet);
    0
}
