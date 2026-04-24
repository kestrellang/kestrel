// test: diagnostics
// stdlib: false

module Test

type A = B // ERROR: circular type alias
type B = C
type C = A
