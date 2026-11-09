// test: diagnostics
// stdlib: false

// Only trailing params with a default may be left off an extension target.
module Test

struct Triple[A, B, C = A] { }

extend Triple[A] { } // ERROR: wrong number of type parameters for 'Triple': expected 3, got 1
extend Triple[A, B] { }
extend Triple[A, B, C, D] { } // ERROR: wrong number of type parameters for 'Triple': expected 3, got 4
