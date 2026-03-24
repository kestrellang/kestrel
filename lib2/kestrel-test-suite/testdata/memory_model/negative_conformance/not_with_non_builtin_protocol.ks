// test: diagnostics
// stdlib: false

module Test

protocol MyProtocol {}

struct Foo: not MyProtocol {} // ERROR: not a language feature protocol
