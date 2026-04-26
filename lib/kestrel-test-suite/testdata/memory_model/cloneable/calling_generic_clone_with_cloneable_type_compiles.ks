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

func makeClone[T](item: T) -> T where T: Cloneable {
    item.clone()
}

func test() -> Data {
    let d = Data(value: 42);
    makeClone(d)
}
