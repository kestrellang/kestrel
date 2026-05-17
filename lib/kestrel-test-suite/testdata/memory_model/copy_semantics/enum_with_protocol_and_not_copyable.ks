// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

protocol Stateful {}

enum Connection: Stateful, not Copyable {
    case Connected
    case Disconnected
}
