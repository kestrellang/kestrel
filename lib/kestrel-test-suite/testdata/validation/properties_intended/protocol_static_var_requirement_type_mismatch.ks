// test: diagnostics
// stdlib: true

module Test

protocol P {
    static var value: std.numeric.Int64
}

struct S: P {
    static var value: std.numeric.Int32 = 0 // ERROR: property 'value' has wrong type for protocol
}
