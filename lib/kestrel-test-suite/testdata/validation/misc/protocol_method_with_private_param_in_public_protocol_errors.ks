// test: diagnostics
// stdlib: false

module Test
private struct Secret { }
public protocol Handler {
    func handle(s: Secret) -> () // ERROR: parameter type in 'handle' is less visible
}
