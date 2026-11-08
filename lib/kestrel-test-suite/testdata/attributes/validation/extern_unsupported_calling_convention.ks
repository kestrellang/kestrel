// test: diagnostics
// stdlib: false

module Test
@extern(.Rust) // ERROR: '.Rust' is not a calling convention; expected '.C'
func external()
