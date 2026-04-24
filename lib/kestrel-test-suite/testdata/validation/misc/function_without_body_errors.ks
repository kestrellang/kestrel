// test: diagnostics
// stdlib: false

module Test
func missingBody() -> lang.i64 // ERROR: requires a body
