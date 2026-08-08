// test: diagnostics
// stdlib: true

// The counterpart to struct_bodyless_accessors_are_storage: because `b` is
// storage, omitting it is an arity error rather than a silent slot shift.

module Main

struct S {
    var a: std.numeric.Int64;
    var b: std.numeric.Int64 { get set }
    var c: std.numeric.Int64;
}

@main
func main() -> lang.i64 {
    let s = S(a: 1, c: 3); // ERROR: but 2 argument(s) were provided
    return 0;
}
