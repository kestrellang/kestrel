// test: diagnostics
// stdlib: false

module Test
@unknownAttribute // WARN: unknown attribute
struct Foo {}
