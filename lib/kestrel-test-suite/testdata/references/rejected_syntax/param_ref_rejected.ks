// test: diagnostics
// stdlib: false

// Stage 0.5: `&T` parses in parameter position but is rejected — and not
// "yet": parameters never take ref types (references-gaps.md §10.6).
// Conventions are the only spelling: `x: T` borrows already.
module Test

func takesRef(x: &lang.i64) { } // ERROR(E480)
