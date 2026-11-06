// test: diagnostics

// A protocol-extension default only satisfies a requirement for types that
// meet the extension's `where` clause. `R` is not Copyable, so
// `extend P where Self: Copyable` gives it no `dup`; `C` is Copyable and
// gets it. The unmet case used to be accepted.

module Test

protocol P { func dup() -> Self }
extend P where Self: Copyable { func dup() -> Self { self } }

struct R: P, not Copyable { var v: Int64 } // ERROR(E454): type 'R' does not implement method 'dup' from protocol 'P'
struct C: P { var v: Int64 }

func use() -> Int64 { C(v: 1).dup().v }
