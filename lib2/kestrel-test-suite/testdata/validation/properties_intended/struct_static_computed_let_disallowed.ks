// test: diagnostics
// stdlib: true

module Test
public struct Foo {
    public static let structStaticComputedLet: std.num.Int64 { 0 } // ERROR: computed properties must use 'var'
}
