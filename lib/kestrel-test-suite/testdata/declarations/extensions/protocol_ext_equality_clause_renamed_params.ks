// test: execution
// stdlib: true
// expect-exit: 0

// G29: a protocol extension's where clause is written in the protocol's own
// parameter names (`A`, `B`); the conformer uses different ones (`X`, `Y`).
// Conformance completeness renames the clause into the conformer's names
// (`substitute_clause`) before asking whether the conformer's own clauses
// entail it. `A.Out = B` must become `X.Out = Y` on BOTH sides of the `=` —
// the subject's root and the RHS param — or the equality can only match a
// clause about the protocol's own parameters, and `describe` is wrongly
// reported missing (E454).
//
// Control: `protocol_ext_equality_clause_renamed_params_control.ks`, where
// the conformer pins `X.Out` to something else and E454 is correct.

module Test

protocol Source {
    type Out
    func produce() -> Out
}

protocol Describable {
    func describe() -> Int64
}

protocol Pairing[A, B] {
    func first() -> A
}

extend Pairing[A, B] where A: Source, A.Out = B {
    public func describe() -> Int64 { 7 }
}

struct Gen: Source {
    type Out = Int64;
    func produce() -> Int64 { 3 }
}

struct Holder[X, Y]: Pairing[X, Y], Describable where X: Source, X.Out = Y {
    var x: X;
    func first() -> X { self.x }
}

@main
func main() -> lang.i32 {
    let h: Holder[Gen, Int64] = Holder(x: Gen());
    if h.describe() != 7 { return 1 }
    0
}
