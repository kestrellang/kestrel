// test: diagnostics
// stdlib: false

module Test

protocol Factory {
    static func create(value value: lang.i64) -> Self
}
func make[T](v: lang.i64) -> T where T: Factory {
    return T.create(value: v)
}
