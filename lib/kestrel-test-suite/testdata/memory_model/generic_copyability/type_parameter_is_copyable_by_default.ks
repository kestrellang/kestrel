// test: diagnostics
// stdlib: true

module Test

func duplicate[T](x: T) -> (T, T) {
    return (x, x)
}
