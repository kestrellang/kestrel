// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Handle: not Copyable, Cloneable { // ERROR: cannot conform to `Cloneable` and opt out of `Copyable`
    var fd: lang.i64

    func clone() -> Handle {
        Handle(fd: self.fd)
    }
}
