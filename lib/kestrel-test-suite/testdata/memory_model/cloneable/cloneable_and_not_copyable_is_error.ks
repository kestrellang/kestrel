// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Invalid: Cloneable, not Copyable { // ERROR: cannot conform to `Cloneable` and opt out of `Copyable`
    var value: lang.i64

    func clone() -> Invalid {
        Invalid(value: self.value)
    }
}
