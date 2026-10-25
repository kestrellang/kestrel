// test: diagnostics
// stdlib: false

module Test
@mian // ERROR: unknown attribute 'mian'; did you mean '@main'?
func entry() {}
