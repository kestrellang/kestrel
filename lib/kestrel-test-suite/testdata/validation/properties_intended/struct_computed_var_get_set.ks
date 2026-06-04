// test: execution
// stdlib: true
// expect-stdout: 5\n9\n

module Main
import std.io.stdio.println

public struct Foo {
    private var _v: std.numeric.Int64

    public var structComputedVar: std.numeric.Int64 {
        get { self._v }
        set { self._v = newValue }
    }

    init() { self._v = 5 }
}

@main
func main() -> lang.i64 {
    var foo = Foo();
    let _ = println(foo.structComputedVar);
    foo.structComputedVar = 9;
    let _ = println(foo.structComputedVar);
    0
}
