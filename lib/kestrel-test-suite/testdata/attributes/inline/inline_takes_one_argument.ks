// test: diagnostics
// stdlib: false

module Test

@inline(.Always, .Never) // ERROR: '@inline' takes at most one argument
func f() {}
