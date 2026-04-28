// test: execution
// stdlib: true
// expect-stdout: 5\n7\n

module Main
import std.io.stdio.println

public struct Foo {
    private static var _s: std.numeric.Int64 = 5;

    public static var structStaticComputedVar: std.numeric.Int64 {
        get { _s }
        set { _s = newValue }
    }
}

func main() -> std.numeric.Int64 {
    let _ = println(Foo.structStaticComputedVar);
    Foo.structStaticComputedVar = 7;
    let _ = println(Foo.structStaticComputedVar);
    0
}
