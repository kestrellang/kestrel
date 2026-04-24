// test: diagnostics
// stdlib: false

module Test

protocol Transformer[Output] {
    func transform() -> Output
    func chain(other: Self) -> Output
}
func apply[T](a: T, b: T) -> lang.i64 where T: Transformer[lang.i64] {
    a.chain(b)
}
