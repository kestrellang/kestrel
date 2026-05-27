// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Inner: Cloneable {
    var value: lang.i64

    func clone() -> Inner {
        Inner(value: self.value)
    }
}

enum Container {
    case Some(value: Inner)
    case None
}
