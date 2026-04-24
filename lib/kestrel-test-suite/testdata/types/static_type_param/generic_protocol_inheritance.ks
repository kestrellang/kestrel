// test: diagnostics
// stdlib: false

module Test

protocol Converter[T] {
    func convert() -> T
}
protocol IntConverter: Converter[lang.i64] {
    func convertTwice() -> lang.i64
}
func useIntConverter[T](val: T) -> lang.i64 where T: IntConverter {
    val.convert()
}
