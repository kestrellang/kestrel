// test: diagnostics
// stdlib: false

module Test
@extern(mangleName: "a", .C) // ERROR: positional argument after a labeled one
func external()
