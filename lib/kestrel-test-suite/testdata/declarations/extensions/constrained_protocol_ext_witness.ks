// test: execution
// stdlib: true
// expect-exit: 0

// #213: a protocol requirement may be witnessed by a CONSTRAINED method on a
// generic protocol extension, when the constraint is satisfied by the
// conformer's concrete binding. `BoxC: Container[Int64]`; the Equatable witness
// `isEqual` is supplied by `extend Container[T] where T: Equatable`, and
// `Int64: Equatable` holds — so `BoxC: Equatable` is satisfied. This is the
// user-level analog of the stdlib's own Array/Slice conformance idiom.
//
// Two halves had to work: the frontend conformance check must recognize the
// witness through the constraint (was a false E454), and mono must instantiate
// the supplying extension's leading type arg (was a "type arg arity mismatch").

module Test

import std.numeric.Int64

protocol Container[T] {
    func item() -> T
}

extend Container[T] where T: Equatable {
    public func isEqual(to other: Self) -> Bool { self.item() == other.item() }
}

struct BoxC: Container[Int64] {
    var v: Int64;
    func item() -> Int64 { self.v }
}

extend BoxC: Equatable { }

func eqG[T](a: T, b: T) -> Bool where T: Equatable { a == b }

@main
func main() -> lang.i32 {
    let a = BoxC(v: 4);
    let b = BoxC(v: 4);
    let c = BoxC(v: 5);
    if a == b { } else { return 10 }   // t1 = true
    if a == c { return 11 }            // t2 = false
    if eqG(a, b) { } else { return 12 } // t3 = true (through a generic Equatable bound)
    0
}
