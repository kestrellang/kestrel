// test: diagnostics
// stdlib: true

module Test
public static let globalStaticLet: std.num.Int64 = 0; // ERROR: properties in global context are already static
