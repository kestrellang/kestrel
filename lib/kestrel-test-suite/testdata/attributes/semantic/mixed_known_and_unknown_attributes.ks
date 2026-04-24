// test: diagnostics
// stdlib: false

module Test
@dummy
@unknown // WARN: unknown attribute
struct Foo {}
