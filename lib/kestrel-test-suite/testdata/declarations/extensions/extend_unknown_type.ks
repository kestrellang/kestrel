// test: diagnostics
// stdlib: false
module Test
extend Unknown { func foo() { } } // ERROR: Unknown
