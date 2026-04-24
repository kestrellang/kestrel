// test: diagnostics
// stdlib: false

module Test
private struct PrivateType { }
public struct Container {
    public let value: PrivateType // ERROR: has type less visible than the field
}
