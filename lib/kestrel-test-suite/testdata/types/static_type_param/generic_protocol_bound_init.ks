// test: diagnostics
// stdlib: false

module Test

protocol Buildable[T] {
    init(value: T)
}
func build[B](v: lang.i64) -> B where B: Buildable[lang.i64] {
    B(v)
}
