// test: execution
// stdlib: true
// expect-stdout: 1\n3\n

module Main
import std.io.stdio.println

enum Foo {
    case A
    private static var _v: std.num.Int64 = 1;

    var computed: std.num.Int64 {
        get { Foo._v }
        set { Foo._v = newValue }
    }
}

func main() -> std.num.Int64 {
    var f: Foo = .A;
    let _ = println(f.computed);
    f.computed = 3;
    let _ = println(f.computed);
    0
}
