// test: diagnostics
// stdlib: false

module Test

protocol Factory[T] {
    static func create() -> T
}
func makeWidget[F]() -> lang.i64 where F: Factory[lang.i64] {
    F.create()
}
