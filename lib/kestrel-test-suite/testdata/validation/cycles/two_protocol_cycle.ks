// test: diagnostics
// stdlib: false

module Test

protocol A: B {} // ERROR
protocol B: A {}
