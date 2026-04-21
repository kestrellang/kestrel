// test: diagnostics
// stdlib: true

module Main

func transform[T, U](x: T, f: (T) -> U) -> U {
    f(x)
}

func test() -> lang.str {
    transform(42, { (n) in "hello" })
}
