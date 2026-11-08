// test: diagnostics
// stdlib: false

module Test
@fileconstant("data.bin") // ERROR: '@fileconstant' cannot be applied to a function; it applies to module-level values
func load() {}
