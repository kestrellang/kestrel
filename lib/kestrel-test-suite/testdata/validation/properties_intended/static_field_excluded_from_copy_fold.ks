// test: execution
// stdlib: true
// expect-stdout: 1\n1\n

// A `static` field is a global, not inline storage, so it must not participate
// in the containing type's copy semantics. Folding it in made `S` NotCopyable
// because of storage no instance of `S` contains, and every second use of an
// `S` value was rejected as a use-after-move (E500).

module Main
import std.io.stdio.println

struct H: not Copyable {
    var v: std.numeric.Int64;
}

struct S {
    static var h: H = H(v: 9);
    var a: std.numeric.Int64;
}

@main
func main() -> lang.i64 {
    let s = S(a: 1);
    let t = s;
    let u = s;
    println(t.a);
    println(u.a);
    return 0;
}
