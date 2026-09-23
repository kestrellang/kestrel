// test: diagnostics
// stdlib: true

// G17 S4 residual: the TYPE-LEVEL spelling of the member-lookup leak. A
// static call written on the projection type, `B.Item.zero()`, reaches the
// solver as a bare `TypeAlias { Producer.Item }` receiver: the base `B` is
// already gone by then, so S4's base filter (which needs a projection
// receiver's base spine) cannot see which receiver it is. The bound
// `A.Item: Zero` is therefore still handed to `B.Item`.
//
// This is worse than the instance-call case S4 fixed. There is no post-mono
// error: the program compiles and crashes with SIGSEGV at run time, because
// `String` has no `zero` witness.
//
// EXPECTED TO FAIL until the type-expression path keeps the projection's
// base. The annotation is the frontend diagnostic that should appear, and it
// turns green when it does. Compare `assoc_projection_bound_member_other_receiver.ks`,
// the instance-call form, which S4 rejects.

module Test

import std.text.String
import std.numeric.Int64

protocol Zero { static func zero() -> Self }
extend Int64: Zero { public static func zero() -> Int64 { 0 } }

protocol Producer { type Item; func produce() -> Item }

struct IntSrc { var v: Int64; }
extend IntSrc: Producer {
    public type Item = Int64;
    public func produce() -> Int64 { self.v }
}
struct StrSrc { var s: String; }
extend StrSrc: Producer {
    public type Item = String;
    public func produce() -> String { self.s.clone() }
}

struct Pair[A, B] where A: Producer, B: Producer, A.Item: Zero {
    var a: A;
    var b: B;
    public func go() -> B.Item { B.Item.zero() } // ERROR: no member 'zero'
}

@main
func main() -> lang.i32 {
    print("[\(Pair(a: IntSrc(v: 7), b: StrSrc(s: "z")).go())]");
    0
}
