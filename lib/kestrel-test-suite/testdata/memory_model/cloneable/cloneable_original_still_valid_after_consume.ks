// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Data: Cloneable {
    var value: lang.i64

    func clone() -> Data {
        Data(value: self.value)
    }
}

func consume(consuming d: Data) {}

func test() {
    let data = Data(value: 42);
    consume(data);
    consume(data)
}
