// test: diagnostics
// stdlib: false

module Test
@builtin("Copyable") // ERROR: expected a builtin like '.Name', found '"Copyable"'
protocol Foo {}
