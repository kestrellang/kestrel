// test: diagnostics
// stdlib: false

module Test

@sugarOnly // ERROR: '@sugarOnly' cannot be applied to a function
func free() {}
