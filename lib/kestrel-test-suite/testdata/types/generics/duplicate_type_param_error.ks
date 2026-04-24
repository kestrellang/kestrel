// test: diagnostics
// stdlib: false

module Test

struct Bad[T, T] {} // ERROR: duplicate type parameter name 'T'
