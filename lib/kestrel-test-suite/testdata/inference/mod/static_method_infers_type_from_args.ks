// test: diagnostics
// stdlib: false

module Main

struct Box[T] {
    var value: T
}

extend Box[T] {
    static func wrap(v: T) -> Box[T] {
        Box[T](value: v)
    }
}

func test() -> lang.i64 {
    let b = Box.wrap(42);
    b.value
}
