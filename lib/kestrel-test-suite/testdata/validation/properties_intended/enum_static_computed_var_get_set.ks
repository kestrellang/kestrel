// test: execution
// stdlib: true
// expect-stdout: 5\n7\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _s: std.num.Int64 = 5;

    static var computed: std.num.Int64 {
        get { _s }
        set { _s = newValue }
    }
}

func main() -> std.num.Int64 {
    let _ = println(Foo.computed);
    Foo.computed = 7;
    let _ = println(Foo.computed);
    0
}
