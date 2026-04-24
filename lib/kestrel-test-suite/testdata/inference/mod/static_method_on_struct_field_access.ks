// test: diagnostics
// stdlib: false

module Main

struct Factory[T] {
    var product: T

    static func make(value: T) -> Factory[T] {
        Factory[T](product: value)
    }
}

func test() -> lang.i64 {
    let f = Factory[lang.i64].make(42);
    f.product
}
