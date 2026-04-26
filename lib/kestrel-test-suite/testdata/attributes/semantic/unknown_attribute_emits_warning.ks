// test: diagnostics
// stdlib: false

module Test
@unknownAttr // WARN: unknown attribute
struct Foo {}
