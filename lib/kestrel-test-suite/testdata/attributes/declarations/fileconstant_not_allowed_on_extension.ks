// test: diagnostics
// stdlib: false

module Test

struct S {}

@fileconstant("x.bin") // ERROR: '@fileconstant' cannot be applied to an extension; it applies to module-level values
extend S {}
