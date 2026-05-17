// test: diagnostics
// stdlib: true

module Test
public struct Foo {
    public let structComputedLet: std.numeric.Int64 { 0 } // ERROR: computed properties must use 'var'
}
