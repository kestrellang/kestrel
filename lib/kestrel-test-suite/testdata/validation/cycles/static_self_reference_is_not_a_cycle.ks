// test: execution
// stdlib: true
// expect-stdout: 1\n

// A `static var` of the enclosing type is a global, not inline storage, so it
// cannot make the type contain itself. The cycle check used `!Callable`, which
// admits statics, and reported a spurious E449.

module Main
import std.io.stdio.println

struct S {
    static var shared: S = S(a: 1);
    var a: std.numeric.Int64;
}

@main
func main() -> lang.i64 {
    let s = S(a: 1);
    println(s.a);
    return 0;
}
