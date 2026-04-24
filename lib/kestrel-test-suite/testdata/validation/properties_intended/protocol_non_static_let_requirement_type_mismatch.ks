// test: diagnostics
// stdlib: true

module Test

protocol P {
    let value: std.num.Int64
}

struct S: P {
    let value: std.num.Int32 // ERROR: property 'value' has wrong type for protocol
}
