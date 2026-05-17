// test: diagnostics
// stdlib: false

module Main

struct Wrapper[T] {
    var inner: T
}

extend Wrapper[T] {
    func rewrap[U](newValue: U) -> Wrapper[U] {
        Wrapper[U](inner: newValue)
    }
}

func test() -> Wrapper[lang.str] {
    let w = Wrapper[lang.i64](inner: 42);
    w.rewrap("hello")
}
