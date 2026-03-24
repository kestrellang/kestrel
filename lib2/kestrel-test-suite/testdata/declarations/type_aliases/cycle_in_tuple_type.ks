// test: diagnostics
// stdlib: false

module Test

type A = (B, lang.i64) // ERROR: circular type alias
type B = A
