// test: diagnostics
// stdlib: false

module Test
@dummy
@unknown // ERROR: unknown attribute
struct Foo {}
