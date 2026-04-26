// test: diagnostics
// stdlib: false

module Test
struct Calculator {
    func compute(x: ()) { }
    func compute(x: ()) { } // ERROR: duplicate function signature
}
