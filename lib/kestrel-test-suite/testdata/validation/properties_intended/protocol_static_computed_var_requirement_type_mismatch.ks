// test: diagnostics
// stdlib: true

module Test

protocol P {
    static var value: std.num.Int64 { get }
}

struct S: P {
    static var value: std.num.Int32 { 0 } // ERROR: property 'value' has wrong type for protocol
}
