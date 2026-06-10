// test: diagnostics
// stdlib: false

// Stage 0.5: nested references parse (`&&` lexes as two ampersands) and
// are rejected with one diagnostic per cluster.
module Test

func f(x: &&lang.i64) { } // ERROR(E487)

func g(y: &mutating &lang.i64) { } // ERROR(E487)
