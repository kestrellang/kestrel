// test: diagnostics
// stdlib: false

module Test
@unknownAttr // ERROR: unknown attribute
struct Foo {}
