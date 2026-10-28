// test: execution
// stdlib: true
// expect-stdout: 1\n2\n3\n

// A bodyless accessor block (`{ get set }`) on a concrete type declares how
// storage is accessed; it does NOT replace the storage. `b` therefore occupies
// a layout slot like any other stored field.
//
// Regression: layout used `!Callable` and memberwise init used `!Computed`,
// which disagree on exactly this form (bodyless blocks get `Computed` but not
// `Callable`). The two rosters drifted and argument 2 was written into `b`'s
// slot, leaving `c` uninitialized, with no diagnostic at any stage.

module Main
import std.io.stdio.println

struct S {
    var a: std.numeric.Int64;
    var b: std.numeric.Int64 { get set }
    var c: std.numeric.Int64;
}

@main
func main() -> lang.i64 {
    let s = S(a: 1, b: 2, c: 3);
    println(s.a);
    println(s.b);
    println(s.c);
    return 0;
}
