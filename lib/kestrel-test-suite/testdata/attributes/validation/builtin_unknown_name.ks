// test: diagnostics
// stdlib: false

module Test
@builtin(.NotABuiltin) // ERROR: unknown builtin '.NotABuiltin'
struct Foo {}
