// test: execution
// stdlib: true
// expect-stdout: 0\n3\n

module Main
import std.io.stdio.println

public struct Foo {
    public var structVar: std.num.Int64 = 0;
}

func main() -> std.num.Int64 {
    var foo = Foo(structVar: 0);
    let _ = println(foo.structVar);
    foo.structVar = 3;
    let _ = println(foo.structVar);
    0
}
