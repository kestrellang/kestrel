// test: diagnostics
// stdlib: false

module Test
@unknown1 // WARN: unknown attribute
@unknown2 // WARN: unknown attribute
struct Foo {}
