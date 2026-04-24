// test: diagnostics
// stdlib: false

module Test

struct Foo {}
struct Foo {} // ERROR: duplicate
