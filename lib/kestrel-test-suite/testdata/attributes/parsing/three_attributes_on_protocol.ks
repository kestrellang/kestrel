// test: diagnostics
// stdlib: false

module Test
@dummy
@dummy("note")
@dummy(enabled: true)
protocol Baz {}
