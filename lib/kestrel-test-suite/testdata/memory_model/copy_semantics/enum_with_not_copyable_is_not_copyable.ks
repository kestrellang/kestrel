// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

enum State: not Copyable {
    case Active
    case Inactive
}
