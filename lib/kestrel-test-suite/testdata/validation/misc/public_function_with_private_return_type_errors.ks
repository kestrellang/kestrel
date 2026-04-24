// test: diagnostics
// stdlib: false

module Test
private struct Secret { }
public func getSecret() -> Secret { } // ERROR: return type of 'getSecret' is less visible
