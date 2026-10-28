// test: execution
// stdlib: true
// expect-stdout: 1\n3\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _v: std.numeric.Int64 = 1;

    var computed: std.numeric.Int64 {
        get { Foo._v }
        set { Foo._v = newValue }
    }
}

@main
func main() -> lang.i64 {
    var f: Foo = .A;
     println(f.computed);
    f.computed = 3;
     println(f.computed);
    0
}
