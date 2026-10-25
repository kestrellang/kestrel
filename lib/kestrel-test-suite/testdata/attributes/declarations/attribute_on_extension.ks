// test: diagnostics
// stdlib: false

module Test

struct S {}

@dummy(1, label: "x")
extend S {}
