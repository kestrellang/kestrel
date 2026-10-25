// test: diagnostics
// stdlib: true

// G29 control for `protocol_ext_equality_clause_renamed_params.ks`. The
// extension still asks for `A.Out = B`, which for this conformer is
// `X.Out = Y`, but the conformer only states `X.Out = Int64`. That does not
// entail the extension's clause, so the extension's `describe` is not
// available and the `Describable` witness is genuinely missing.

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

struct Holder[X, Y]: Pairing[X, Y], Describable where X: Source, X.Out = Int64 { // ERROR: type 'Holder' does not implement method 'describe' from protocol 'Describable'
    var x: X;
    func first() -> X { self.x }
}
