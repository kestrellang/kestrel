// test: diagnostics
// stdlib: true

module Test
public let globalComputedLet: std.numeric.Int64 { 0 } // ERROR: computed properties must use 'var'
