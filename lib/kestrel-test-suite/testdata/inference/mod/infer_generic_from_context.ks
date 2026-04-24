// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    var value: T
}

func test() {
    let b = Box(value: 42);
    let x: lang.i64 = b.value;
}
