// test: diagnostics
// stdlib: true

module Test
public static var globalStaticComputedVar: std.numeric.Int64 { 0 } // ERROR: computed properties in global context are already static
