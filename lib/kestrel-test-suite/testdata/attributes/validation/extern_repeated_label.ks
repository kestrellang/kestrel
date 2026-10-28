// test: diagnostics
// stdlib: false

module Test
@extern(.C, mangleName: "a", mangleName: "b") // ERROR: argument 'mangleName' given twice
func external()
