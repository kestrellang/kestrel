// test: diagnostics
// stdlib: true

module Test

protocol P {
    var value: std.num.Int64
}

struct S: P {
    var value: std.num.Int32 // ERROR: property 'value' has wrong type for protocol
}
