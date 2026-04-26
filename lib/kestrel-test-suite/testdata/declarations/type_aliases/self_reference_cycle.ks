// test: diagnostics
// stdlib: false

module Test

type A = A // ERROR: circular type alias
