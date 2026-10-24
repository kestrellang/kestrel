// test: diagnostics
// stdlib: false

module Test
@builtin(42) // ERROR: expected a builtin like '.Name', found '42'
protocol Foo {}
