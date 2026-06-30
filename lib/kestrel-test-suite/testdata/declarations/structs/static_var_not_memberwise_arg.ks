// test: diagnostics
// stdlib: true
//
// #150: passing a static stored var as a memberwise-init argument must be
// rejected cleanly (it is not an instance field), not silently accepted or
// crashed in codegen. `S` has exactly one instance field, so the two-argument
// call is an arity error.

module Test

struct S {
    var v: Int64;
    static var sv: Int64 = 5;
}

@main
func main() -> lang.i32 {
    let s = S(v: 0, sv: 77); // ERROR: struct 'S' has 1 field(s), but 2 argument(s) were provided
    return 0;
}
