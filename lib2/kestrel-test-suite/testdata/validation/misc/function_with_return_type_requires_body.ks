// test: diagnostics
// stdlib: false

module Test
func valid() { }
func invalid() -> lang.i64 // ERROR: 'invalid' requires a body
