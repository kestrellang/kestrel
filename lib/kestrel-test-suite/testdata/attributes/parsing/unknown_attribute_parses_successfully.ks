// test: diagnostics
// stdlib: false

module Test
@unknownAttribute // ERROR: unknown attribute
struct Foo {}
