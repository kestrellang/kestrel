// test: diagnostics
// stdlib: false

module Test
@first
@second("arg")
@third(a: 1, b: 2)
struct Foo {}
