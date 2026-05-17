// test: diagnostics
// stdlib: false

module Test

struct Config[A, B, C = lang.str] { }
type BadConfig = Config[lang.i1]; // ERROR: too few type arguments
