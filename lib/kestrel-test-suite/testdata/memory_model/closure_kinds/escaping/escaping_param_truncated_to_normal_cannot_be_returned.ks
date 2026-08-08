// test: diagnostics
// stdlib: true

// Plan D5, representation conversion (2) (docs/plans/closure-kinds/
// closure-kinds-plan.md): `escaping -> normal` is a TRUNCATING VIEW
// (`{fn, handle-as-ptr}`), not a re-labelled handle — the value is frame-bound
// and rooted at the source handle's local. When the source is a BORROWED
// PARAMETER, that truncated view must not be returnable: `check_escapes`
// rejects a closure-carrier return whose root is a borrow-param when the
// return slot's kind is a view kind.
//
// Returning the same parameter AT `escaping` is legal (an owned, retained
// copy) — see the sibling escaping_param_returned_as_escaping_allowed.ks.
// MIR-stage code: this file must stay free of analyzer-stage errors, so E494
// on the tail is the ONLY diagnostic here.
module Test

import std.numeric.Int64

func take(f: escaping () -> Int64) -> () -> Int64 {
    f // ERROR(E494)
}
