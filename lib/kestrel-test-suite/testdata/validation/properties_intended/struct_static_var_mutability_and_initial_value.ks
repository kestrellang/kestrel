// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

module Main
import std.io.stdio.println

public struct Foo {
    public static var structStaticVar: std.numeric.Int64 = 1;
}

func main() -> std.numeric.Int64 {
    let _ = println(Foo.structStaticVar);
    Foo.structStaticVar = 2;
    let _ = println(Foo.structStaticVar);
    0
}
