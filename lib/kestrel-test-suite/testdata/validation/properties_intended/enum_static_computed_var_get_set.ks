// test: execution
// stdlib: true
// expect-stdout: 5\n7\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _s: std.numeric.Int64 = 5;

    static var computed: std.numeric.Int64 {
        get { _s }
        set { _s = newValue }
    }
}

@main
func main() -> lang.i64 {
    let _ = println(Foo.computed);
    Foo.computed = 7;
    let _ = println(Foo.computed);
    0
}
