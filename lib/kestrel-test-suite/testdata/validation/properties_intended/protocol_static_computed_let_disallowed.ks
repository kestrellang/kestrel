// test: diagnostics
// stdlib: true

module Test

protocol P {
    static let value: std.numeric.Int64 { get } // ERROR: computed properties must use 'var'
}
