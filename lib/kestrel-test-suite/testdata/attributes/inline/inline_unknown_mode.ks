// test: diagnostics
// stdlib: false

module Test

@inline(.Sometimes) // ERROR: '.Sometimes' is not an inline mode; expected '.Always' or '.Never'
func f() {}
