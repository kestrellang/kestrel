// test: diagnostics
// stdlib: false

module Test
@first // ERROR: unknown attribute
@second("arg") // ERROR: unknown attribute
@third(a: 1, b: 2) // ERROR: unknown attribute
struct Foo {}
