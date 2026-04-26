// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    @builtin(.Clone)
    func clone() -> Self
}

struct Data: Cloneable {
    var value: lang.i64

    func clone() -> Data {
        Data(value: self.value)
    }
}

func borrow(d: Data) {}

func test() {
    let data = Data(value: 42);
    borrow(data)
}
