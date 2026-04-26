// test: diagnostics
// stdlib: true

module Test
public let globalComputedLet: std.num.Int64 { 0 } // ERROR: computed properties must use 'var'
