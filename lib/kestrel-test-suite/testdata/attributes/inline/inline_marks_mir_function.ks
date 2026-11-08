// test: mir
// stdlib: false
// mir-filter: Test.

// `@inline` becomes a typed field on the MIR function (shown in the dump
// header); codegen reads it, never the attribute.

module Test

@inline(.Always)
func always() {}

@inline(.Never)
func never() {}

@inline
func hint() {}

func plain() {}
