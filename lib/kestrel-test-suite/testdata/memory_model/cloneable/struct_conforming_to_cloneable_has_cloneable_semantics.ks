// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct MyData: Cloneable {
    var value: lang.i64

    func clone() -> MyData {
        MyData(value: self.value)
    }
}
