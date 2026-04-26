// test: diagnostics
// stdlib: false

module Test

struct Plain { }
type Bad = Plain[lang.i64]; // ERROR: does not accept type arguments
