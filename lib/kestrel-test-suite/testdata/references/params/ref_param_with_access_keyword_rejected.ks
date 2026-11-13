// test: diagnostics
// stdlib: false

// A reference-typed parameter spells its access mode with the type:
// `x: &T` borrows, `x: &mutating T` mutably borrows. Adding a `mutating` /
// `consuming` keyword on top says it twice, so it is rejected (E480).
// The plain forms, in declarations and function types, are accepted.
module Test

func takesRef(x: &lang.i64) { }
func takesMutRef(x: &mutating lang.i64) { }
func takesFn(g: (&lang.i64) -> lang.i64, h: (&mutating lang.i64) -> lang.i64) { }

func mutatingRef(mutating x: &lang.i64) { } // ERROR(E480)
func consumingRef(consuming x: &lang.i64) { } // ERROR(E480)
func mutatingFnRef(g: (mutating &lang.i64) -> lang.i64) { } // ERROR(E480)
