// test: diagnostics
// stdlib: false

module Test

@inline // ERROR: '@inline' cannot be applied to a struct; it applies to functions, methods, protocol requirements, initializers, deinits, subscripts and computed properties
struct S {}
