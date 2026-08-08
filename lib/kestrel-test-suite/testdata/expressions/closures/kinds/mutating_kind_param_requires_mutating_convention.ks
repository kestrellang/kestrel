// test: diagnostics
// stdlib: false

// "The kind also dictates the parameter convention needed to call it: a
// `mutating (T) -> U` parameter must itself be `mutating`" — docs/design/
// closures.md, "Passing: What Fits Where".
//
// Plan D8 (docs/plans/closure-kinds/closure-kinds-plan.md) splits the code
// three ways: the KIND/CONVENTION PAIRING is a signature-level fact and gets
// its own DeclCheck code, E625 `closure_kind_convention_pairing` (a bodiless
// declaration never generates a Coerce, so it can never reach E624's
// coerce-path reporting). Calling a `mutating`-kind closure held in a `let` is
// a different failure again — it is a mutating USE of the callee binding, so
// it routes through the existing mutability band (`classify_mutability`) and
// lands in the E203 family. E624 is reserved for passing-table rejections.
module Test

// Borrowing (default) convention cannot supply the exclusive access a
// `mutating` kind needs.
func borrowedMutatingKind(f: mutating (lang.i64) -> lang.i64) -> lang.i64 { 0 } // ERROR(E625)

// `consuming` is not `mutating` either — the pairing is exact, not "at least".
func consumedMutatingKind(consuming f: mutating (lang.i64) -> lang.i64) -> lang.i64 { 0 } // ERROR(E625)

// Non-exclusive call: a `mutating` closure must live in a `var`.
func callFromLet(x: lang.i64) -> lang.i64 {
    let m: mutating () -> lang.i64 = { x };
    m() // ERROR(E203)
}
