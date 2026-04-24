// test: diagnostics
// stdlib: false

module Test

protocol A { }
protocol B { }
protocol C { }
struct Complex[X, Y, Z] where X: A, Y: B and C, Z: A { }
