// test: diagnostics
// stdlib: false

module Test
struct Wrapper[T] {
    var value: T
}
func wrap[T](value: T) -> Wrapper[T] {
    Wrapper(value: value)
}
func test() -> Wrapper[lang.i64] {
    wrap(42)
}
