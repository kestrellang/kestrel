// test: diagnostics
// stdlib: false

module Test
func mixed_types() { [1, "hello", true] } // ERROR: type mismatch
