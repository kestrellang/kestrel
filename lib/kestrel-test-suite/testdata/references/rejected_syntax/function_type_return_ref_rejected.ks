// test: diagnostics
// stdlib: false

// Stage 0.5: `() -> &T` as a function type is the ref-returning
// function-value backdoor (stage-1 E-REF-16 territory) — rejected here.
module Test

func takesFn(g: () -> &lang.i64) { } // ERROR(E486)
