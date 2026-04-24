// test: diagnostics
// stdlib: false

module Main

func apply[T, U](f: (T) -> U, x: T) -> U {
    f(x)
}
