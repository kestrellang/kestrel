// test: diagnostics
// stdlib: false

module Test
@extern(.C, mangle: "ext") // ERROR: '@extern' has no argument 'mangle'; did you mean 'mangleName'?
func external()
