// test: diagnostics
// stdlib: false

module Test
enum State {
    case active

    func describe(x: ()) { }
    func describe(x: ()) { } // ERROR: duplicate function signature
}
