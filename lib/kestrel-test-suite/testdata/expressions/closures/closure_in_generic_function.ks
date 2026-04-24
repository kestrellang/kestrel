// test: diagnostics
// stdlib: false

module Main

func identity[T](x: T, f: (T) -> T) -> T {
    f(x)
}

func test() -> lang.i64 {
    identity(10, { it })
}
