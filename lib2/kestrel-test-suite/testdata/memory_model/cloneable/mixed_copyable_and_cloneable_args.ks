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

struct Point {
    var x: lang.i64
    var y: lang.i64
}

struct Data: Cloneable {
    var value: lang.i64

    func clone() -> Data {
        Data(value: self.value)
    }
}

func consumeMixed(consuming p: Point, consuming d: Data) {}

func test() {
    let pt = Point(x: 1, y: 2);
    let data = Data(value: 42);
    consumeMixed(pt, data)
}
