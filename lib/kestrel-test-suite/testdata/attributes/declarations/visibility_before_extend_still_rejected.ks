// test: diagnostics
// stdlib: false

// `extend` takes attributes but no visibility keyword.

module Test

struct S {}

public extend S {} // ERROR: expected
