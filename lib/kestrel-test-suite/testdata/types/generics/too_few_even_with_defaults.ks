// test: diagnostics
// stdlib: false

module Test

struct Triple[A, B, C = lang.i64] { }
type Bad = Triple[lang.i64]; // ERROR: too few type arguments
