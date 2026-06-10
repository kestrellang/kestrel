// test: diagnostics
// stdlib: false

// Stage 0.5: `&mutating T` in parameter position is a second spelling of
// `mutating x: T` — rejected permanently (references-gaps.md §10.6).
module Test

func takesMutRef(x: &mutating lang.i64) { } // ERROR(E480)
