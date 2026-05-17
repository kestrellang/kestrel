// test: diagnostics
// stdlib: false

module Test
struct Logger {
    func log(message msg: ()) { }
    func log(error err: ()) { }
    func log(warning warn: ()) { }
}
