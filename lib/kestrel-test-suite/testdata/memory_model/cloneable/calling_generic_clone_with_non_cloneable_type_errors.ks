// test: diagnostics
// stdlib: false

module Test

@builtin(.Copyable)
protocol Copyable {}

@builtin(.Cloneable)
protocol Cloneable: Copyable {
    func clone() -> Self
}

struct Point {
    var x: lang.i64
    var y: lang.i64
}

func makeClone[T](item: T) -> T where T: Cloneable {
    item.clone()
}

func test() -> Point {
    let p = Point(x: 1, y: 2);
    makeClone(p) // ERROR:
}
