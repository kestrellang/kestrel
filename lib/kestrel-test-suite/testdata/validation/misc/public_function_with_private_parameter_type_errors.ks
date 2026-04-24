// test: diagnostics
// stdlib: false

module Test
private struct Secret { }
public func process(s: Secret) { } // ERROR: parameter type in 'process' is less visible
