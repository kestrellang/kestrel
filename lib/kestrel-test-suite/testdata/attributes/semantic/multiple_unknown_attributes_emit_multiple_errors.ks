// test: diagnostics
// stdlib: false

module Test
@unknown1 // ERROR: unknown attribute
@unknown2 // ERROR: unknown attribute
struct Foo {}
