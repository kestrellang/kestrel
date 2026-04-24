// test: diagnostics
// stdlib: true

module Test
public static let globalStaticComputedLet: std.num.Int64 { 0 } // ERROR: computed properties must use 'var'
