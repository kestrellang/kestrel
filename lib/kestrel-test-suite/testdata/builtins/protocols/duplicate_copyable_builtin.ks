// test: diagnostics
// stdlib: false

// Two declarations claiming the same lang item is an error (E400); the first
// declaration in source order stays the lang item. Before the frontend
// rewrite the last annotation silently won.
module Test
@builtin(.Copyable)
protocol Copyable1 {}

@builtin(.Copyable)
protocol Copyable2 {} // ERROR: duplicate @builtin(.Copyable): already declared by 'Copyable1'
