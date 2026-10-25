// test: execution
// stdlib: true
// expect-exit: 0

// G13 / Bug 1 (false reject). Sibling of `constrained_protocol_ext_witness.ks`
// (the `where T: Equatable` control) and
// `protocol_ext_witness_gated_by_cloneable_bound.ks` — read the three together:
// the SAME program shape must behave identically whichever protocol names the
// bound. Before the fix only `Equatable` worked.
//
// `BoxC: Container[Int64]`; the `Dup` witness `dup` is supplied by
// `extend Container[T] where T: Copyable`, and `Int64` IS Copyable — so
// `extend BoxC: Dup { }` is complete. It used to be
//   error[E454]: type 'BoxC' does not implement method 'dup' from protocol 'Dup'
// because the conformance-completeness analyzer evaluated the bound through
// `type_satisfies`, which routed `Copyable` into `ConformingProtocols`.
// `ConformingProtocols` only materializes EXPLICIT conformances plus
// inheritance, so it never reports the implicit `Copyable` that every default
// type carries, and the bound read as unsatisfied. Meanwhile the solver
// answered the same question `true` via the copy-semantics classifier, so
// calling `a.dup()` directly (with the `Dup` conformance removed) compiled
// fine — the analyzer was the only component that disagreed.
//
// `type_satisfies` now answers Copyable/Cloneable with that same classifier.
//
// The assertion is on the RESULT, not merely on E454 being gone: a witness
// that resolves but instantiates the wrong extension would still print the
// wrong value.

module Test

import std.numeric.Int64

protocol Dup {
    func dup() -> Self
}

protocol Container[T] {
    func item() -> T
}

extend Container[T] where T: Copyable {
    public func dup() -> Self { self }
}

struct BoxC: Container[Int64] {
    var v: Int64;
    func item() -> Int64 { self.v }
}

extend BoxC: Dup { }

// Reaching the witness through a generic `Dup` bound exercises the mono path
// as well as the frontend completeness check.
func dupG[T](value: T) -> T where T: Dup { value.dup() }

@main
func main() -> lang.i32 {
    let a = BoxC(v: 4);

    let b = a.dup();
    if b.item() != 4 { return 10 }

    let c = dupG(BoxC(v: 7));
    if c.item() != 7 { return 11 }

    0
}
