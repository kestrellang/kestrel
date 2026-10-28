// test: diagnostics
// stdlib: true

// docs/design/closures.md: "The kind also dictates the parameter convention
// needed to call it: ... a `consuming (T) -> U` parameter must be `consuming`.
// The compiler enforces the pairing." A borrowing (default) parameter cannot
// hold a one-shot closure it is allowed to consume.
//
// The pairing is a SIGNATURE-level fact, so plan D8 (docs/plans/closure-kinds/
// closure-kinds-plan.md) gives it its own DeclCheck code, E625
// `closure_kind_convention_pairing`; E624 is reserved for passing-table
// rejections reported on the coerce path.
module Test

import std.numeric.Int64

func onDoneBorrowing(f: consuming () -> ()) { f(); } // ERROR(E625)

func main() -> lang.i64 { 0 }
