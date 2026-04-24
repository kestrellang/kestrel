// test: diagnostics
// stdlib: false

module Test

func make[T]() -> T {
    return T() // ERROR:
}
