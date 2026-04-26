// test: diagnostics
// stdlib: false

module Test

protocol Hashable {
    func hash() -> lang.i64
}

struct NotHashable {
    var value: lang.i64
}

struct Container {
    var hash: lang.i64

    init[T](item: T) where T: Hashable {
        self.hash = item.hash()
    }
}

func test() -> Container {
    let n = NotHashable(value: 42);
    Container(n) // ERROR: Hashable
}
