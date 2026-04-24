// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    var value: T
}

func test() -> lang.i64 {
    let b = Box(value: 42);
    b.value
}
