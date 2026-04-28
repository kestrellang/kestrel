// test: diagnostics
// stdlib: true

module Test
public static let globalStaticComputedLet: std.numeric.Int64 { 0 } // ERROR: computed properties must use 'var'
