// test: diagnostics
// stdlib: true

module Test

protocol P {
    let value: std.numeric.Int64
}

struct S: P {
    let value: std.numeric.Int32 // ERROR: property 'value' has wrong type for protocol
}
