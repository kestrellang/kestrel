// test: diagnostics
// stdlib: false

// Without the `@builtin(.Static)` protocol in scope, `not Static` is just a
// negative conformance to a regular protocol — rejected like any other.
// There is deliberately NO name-based fallback for Static (unlike the
// legacy Copyable string-match carve): stdlib-less fixtures must inline
// the builtin.

module Test

protocol Static {}

struct Handle: not Static {} // ERROR: not a language feature protocol
