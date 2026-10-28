// test: execution
// stdlib: true
// expect-stdout: 11\n

module Main
import std.io.stdio.println

public struct Foo {
    public let structLet: std.numeric.Int64 = 11;
}

@main
func main() -> lang.i64 {
    let foo = Foo(structLet: 11);
     println(foo.structLet);
    0
}
