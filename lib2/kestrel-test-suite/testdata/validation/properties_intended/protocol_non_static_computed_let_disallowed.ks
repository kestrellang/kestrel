// test: diagnostics
// stdlib: true

module Test

protocol P {
    let value: std.num.Int64 { get } // ERROR: computed properties must use 'var'
}
