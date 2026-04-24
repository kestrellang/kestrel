// test: diagnostics
// stdlib: false

module Test

type Simple = lang.i64;
type Bad = Simple[lang.str]; // ERROR: does not accept type arguments
