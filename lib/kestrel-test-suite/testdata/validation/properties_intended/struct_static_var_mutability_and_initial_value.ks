// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

module Main
import std.io.stdio.println

public struct Foo {
    public static var structStaticVar: std.numeric.Int64 = 1;
}

@main
func main() -> lang.i64 {
     println(Foo.structStaticVar);
    Foo.structStaticVar = 2;
     println(Foo.structStaticVar);
    0
}
