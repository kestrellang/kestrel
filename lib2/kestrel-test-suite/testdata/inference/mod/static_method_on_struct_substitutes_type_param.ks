// test: diagnostics
// stdlib: false

module Main

struct Factory[T] {
    var product: T

    static func make(value: T) -> Factory[T] {
        Factory[T](product: value)
    }
}

func test() -> Factory[lang.i64] {
    Factory[lang.i64].make(42)
}
