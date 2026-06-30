// test: execution
// stdlib: true
// expect-exit: 0
//
// #150: a static stored var must NOT count as a memberwise-init field. The
// synthesized memberwise init for `S` takes only the instance field `v`; the
// static `sv` is reachable through the type. A regression here reported
// "struct 'S' has 2 field(s)" for the one-field `S(v:)` call and later indexed
// past the 1-field layout in codegen.

module Test

struct S {
    var v: Int64;
    static var sv: Int64 = 5;
}

@main
func main() -> lang.i32 {
    let s = S(v: 0);          // one-field memberwise init
    if s.v != 0 { return 1 }
    if S.sv != 5 { return 2 } // static read through the type
    S.sv = 9;
    if S.sv != 9 { return 3 } // static write through the type
    0
}
