// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

protocol Resource {}

struct Handle: not Copyable, Resource {}
