// test: diagnostics
// stdlib: false

module Test

enum D: not Copyable {
    case a
    deinit { }
    deinit { } // ERROR: enum `D` already has a deinit
}
