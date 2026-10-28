// test: diagnostics
// stdlib: true

// With the standard library loaded, only `std` may declare lang items. A user
// `@builtin` is E401 and is ignored (it never enters the builtin index), so it
// cannot replace the stdlib's `Copyable`. Programs built without a stdlib
// (`// stdlib: false`, `--no-std`) still declare their own lang items.

module Test

@builtin(.Copyable) // ERROR: @builtin(.Copyable) is reserved for the standard library
protocol MyCopyable {}
