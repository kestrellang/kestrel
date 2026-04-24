// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    var value: T

    func read() -> T { self.value }
}

func test() -> lang.i64 {
    let b = Box(value: 42);
    b.read()
}
