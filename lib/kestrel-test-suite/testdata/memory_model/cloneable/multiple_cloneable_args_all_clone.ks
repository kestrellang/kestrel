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

func consumeTwo(consuming a: Data, consuming b: Data) {}

func test() {
    let d1 = Data(value: 1);
    let d2 = Data(value: 2);
    consumeTwo(d1, d2)
}
