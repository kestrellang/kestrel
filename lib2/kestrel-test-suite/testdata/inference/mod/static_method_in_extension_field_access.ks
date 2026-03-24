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
    let b = Box[lang.i64].wrap(42);
    b.value
}
