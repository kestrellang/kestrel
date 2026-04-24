// test: diagnostics
// stdlib: false

module Test
static func topLevel() { } // ERROR: cannot be static in this context
