// test: execution
// stdlib: true
// expect-stdout: 1\n2\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    static var staticVar: std.num.Int64 = 1;
}

func main() -> std.num.Int64 {
    let _ = println(Foo.staticVar);
    Foo.staticVar = 2;
    let _ = println(Foo.staticVar);
    0
}
