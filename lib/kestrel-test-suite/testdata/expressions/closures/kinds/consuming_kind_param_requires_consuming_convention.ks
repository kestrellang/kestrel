// test: diagnostics
// stdlib: false

// "a `consuming (T) -> U` parameter must be `consuming`. The compiler enforces
// the pairing." (docs/design/closures.md — "Passing: What Fits Where".)
// Calling a one-shot closure consumes it, so neither the borrowing default nor
// `mutating` can supply the ownership the kind requires.
//
// The pairing is a SIGNATURE-level fact, so plan D8 (docs/plans/closure-kinds/
// closure-kinds-plan.md) gives it its own DeclCheck code, E625
// `closure_kind_convention_pairing` — a bodiless declaration never generates a
// Coerce, so it can never reach E624, which is now reserved for passing-table
// rejections.
module Test

func borrowedConsumingKind(f: consuming (lang.i64) -> lang.i64) -> lang.i64 { 0 } // ERROR(E625)

func mutatedConsumingKind(mutating f: consuming (lang.i64) -> lang.i64) -> lang.i64 { 0 } // ERROR(E625)
