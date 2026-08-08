// test: diagnostics
// stdlib: false

// docs/design/closures.md: "the kind also dictates the parameter convention
// needed to call it: a `mutating (T) -> U` parameter must itself be
// `mutating`". A borrowing (default-convention) parameter of mutating-kind
// type cannot supply exclusive access, so the pairing is rejected.
//
// The pairing is a SIGNATURE-level fact, so plan D8 (docs/plans/closure-kinds/
// closure-kinds-plan.md) gives it its own DeclCheck code, E625
// `closure_kind_convention_pairing` — this declaration has no body and so
// never generates a Coerce, which is where E624 (the passing table) reports.
module Test

func each(action: mutating (lang.i64) -> ()) { } // ERROR(E625)
