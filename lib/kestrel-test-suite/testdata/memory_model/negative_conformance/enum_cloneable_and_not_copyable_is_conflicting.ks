// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

protocol Cloneable: Copyable {
    func clone() -> Self
}

enum State: Cloneable, not Copyable { // ERROR: cannot conform to `Cloneable` and opt out of `Copyable`
    case Active
    case Inactive

    func clone() -> State {
        self
    }
}
