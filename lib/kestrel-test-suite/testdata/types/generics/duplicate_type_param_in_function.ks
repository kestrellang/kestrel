// test: diagnostics
// stdlib: false

module Test

func bad[A, A]() { } // ERROR: duplicate type parameter name 'A'
