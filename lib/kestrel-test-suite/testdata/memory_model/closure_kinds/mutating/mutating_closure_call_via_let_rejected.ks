// test: diagnostics
// stdlib: false

// docs/design/closures.md: calling a `mutating`-kind closure is an exclusive
// use, so the value must live in a `var` (or a `mutating` parameter).
//
// Plan D8 (docs/plans/closure-kinds/closure-kinds-plan.md) routes this through
// the EXISTING mutability band rather than the kind machinery: calling a
// mutating-kind closure is a mutating use of the callee binding, so
// `classify_mutability` sees an immutable `let` handed to a mutating position
// and the E203 family reports it. E624 is reserved for passing-table
// rejections; E625 for the signature-level kind/convention pairing.
module Test

func test() {
    var total = 0;
    let bump: mutating () -> () = { total = lang.i64_add(total, 1); };
    bump(); // ERROR(E203)
}
