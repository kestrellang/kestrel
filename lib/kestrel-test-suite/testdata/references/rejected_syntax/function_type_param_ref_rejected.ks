// test: diagnostics
// stdlib: false

// Stage 0.5: function types follow the same permanent parameter rule —
// `(mutating T) -> R` (#106) is the spelling, `(&mutating T) -> R` is not.
module Test

func takesFn(g: (&mutating lang.i64) -> lang.i64) { } // ERROR(E480)
