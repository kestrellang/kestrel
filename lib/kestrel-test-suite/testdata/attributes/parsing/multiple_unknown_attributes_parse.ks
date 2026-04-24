// test: diagnostics
// stdlib: false

module Test
@first // WARN: unknown attribute
@second("arg") // WARN: unknown attribute
@third(a: 1, b: 2) // WARN: unknown attribute
struct Foo {}
